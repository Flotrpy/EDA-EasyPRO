//! Copper connectivity on the board and the ratsnest of unrouted links.

use crate::geom::{Point, Rect};
use crate::library::Library;
use crate::pcb::{CopperItem, CopperRef, Pcb};
use rstar::primitives::{GeomWithData, Rectangle};
use rstar::{RTree, AABB};
use std::collections::{BTreeMap, HashMap};

pub type ItemTree = RTree<GeomWithData<Rectangle<[i64; 2]>, usize>>;

pub fn index(items: &[CopperItem]) -> ItemTree {
    RTree::bulk_load(
        items
            .iter()
            .enumerate()
            .map(|(i, it)| {
                let b = it.hull.bbox();
                GeomWithData::new(
                    Rectangle::from_corners([b.min.x, b.min.y], [b.max.x, b.max.y]),
                    i,
                )
            })
            .collect(),
    )
}

/// Candidate pairs whose boxes, grown by `margin`, overlap (i < j).
pub fn near_pairs(items: &[CopperItem], tree: &ItemTree, margin: i64) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let b = it.hull.bbox().inflate(margin);
        let env = AABB::from_corners([b.min.x, b.min.y], [b.max.x, b.max.y]);
        for g in tree.locate_in_envelope_intersecting(&env) {
            let j = g.data;
            if j > i && it.layer.shares_copper(items[j].layer) {
                out.push((i, j));
            }
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
pub struct RatLine {
    pub net: String,
    pub a: Point,
    pub b: Point,
}

#[derive(Clone, Debug, Default)]
pub struct BoardConnectivity {
    pub ratsnest: Vec<RatLine>,
    /// Net -> number of missing connections.
    pub unrouted: BTreeMap<String, usize>,
}

fn anchors(it: &CopperItem) -> Vec<Point> {
    match it.what {
        CopperRef::Pad { .. } | CopperRef::Via(_) => {
            vec![crate::geom::Rect::from_points(it.hull.points.iter().copied()).center()]
        }
        CopperRef::Track(_) => it.hull.points.clone(),
        CopperRef::Pour(..) => vec![],
    }
}

/// Group same-net copper that touches and connect the groups with a
/// minimum spanning tree of airwires.
pub fn analyze(pcb: &Pcb, lib: &Library) -> BoardConnectivity {
    let items = pcb.copper_items(lib);
    analyze_items(&items)
}

pub fn analyze_items(items: &[CopperItem]) -> BoardConnectivity {
    let tree = index(items);
    let mut parent: Vec<usize> = (0..items.len()).collect();
    fn find(p: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        let mut c = x;
        while p[c] != r {
            let n = p[c];
            p[c] = r;
            c = n;
        }
        r
    }
    for (i, j) in near_pairs(items, &tree, 0) {
        if items[i].net.is_empty() || items[i].net != items[j].net {
            continue;
        }
        if items[i].hull.distance(&items[j].hull) <= 0.0 {
            let (a, b) = (find(&mut parent, i), find(&mut parent, j));
            parent[a] = b;
        }
    }

    // net -> cluster root -> anchor points (only clusters containing pads).
    let mut nets: HashMap<&str, BTreeMap<usize, Vec<Point>>> = HashMap::new();
    let mut has_pad: HashMap<usize, bool> = HashMap::new();
    for (i, it) in items.iter().enumerate() {
        if it.net.is_empty() {
            continue;
        }
        let r = find(&mut parent, i);
        if matches!(it.what, CopperRef::Pad { .. }) {
            has_pad.insert(r, true);
        }
        nets.entry(it.net.as_str())
            .or_default()
            .entry(r)
            .or_default()
            .extend(anchors(it));
    }

    let mut out = BoardConnectivity::default();
    let mut names: Vec<_> = nets.keys().copied().collect();
    names.sort();
    for name in names {
        let clusters: Vec<Vec<Point>> = nets[name]
            .iter()
            .filter(|(r, pts)| has_pad.get(r).copied().unwrap_or(false) && !pts.is_empty())
            .map(|(_, pts)| pts.clone())
            .collect();
        if clusters.len() < 2 {
            continue;
        }
        out.unrouted.insert(name.to_string(), clusters.len() - 1);
        // Prim's algorithm over clusters with closest-point distances.
        let n = clusters.len();
        let mut in_tree = vec![false; n];
        let mut best: Vec<(f64, Point, Point)> = vec![(f64::INFINITY, Point::ZERO, Point::ZERO); n];
        in_tree[0] = true;
        let closest = |a: &[Point], b: &[Point]| {
            let mut m = (f64::INFINITY, Point::ZERO, Point::ZERO);
            for &p in a {
                for &q in b {
                    let d = p.dist(q);
                    if d < m.0 {
                        m = (d, p, q);
                    }
                }
            }
            m
        };
        for j in 1..n {
            best[j] = closest(&clusters[0], &clusters[j]);
        }
        for _ in 1..n {
            let (k, _) = (0..n)
                .filter(|&j| !in_tree[j])
                .map(|j| (j, best[j].0))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            in_tree[k] = true;
            out.ratsnest.push(RatLine {
                net: name.to_string(),
                a: best[k].1,
                b: best[k].2,
            });
            for j in 0..n {
                if !in_tree[j] {
                    let c = closest(&clusters[k], &clusters[j]);
                    if c.0 < best[j].0 {
                        best[j] = c;
                    }
                }
            }
        }
    }
    out
}

/// Bounding box of an item, handy for zoom-to.
pub fn item_bbox(it: &CopperItem) -> Rect {
    it.hull.bbox()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Id;
    use crate::layer::Layer;
    use crate::library::Hull;
    use crate::pcb::CopperRef;

    fn pad(id: u64, x: i64, net: &str) -> CopperItem {
        CopperItem {
            what: CopperRef::Pad {
                comp: Id(id),
                pad: "1".into(),
            },
            net: net.into(),
            layer: Layer::TopCopper,
            hull: Hull::circle(Point::new(x, 0), 10),
        }
    }

    #[test]
    fn ratsnest_until_routed() {
        let mut items = vec![
            pad(1, 0, "A"),
            pad(2, 100, "A"),
            pad(3, 200, "A"),
            pad(4, 300, "B"),
        ];
        let c = analyze_items(&items);
        assert_eq!(c.unrouted.get("A"), Some(&2));
        assert_eq!(c.ratsnest.len(), 2);
        items.push(CopperItem {
            what: CopperRef::Track(Id(9)),
            net: "A".into(),
            layer: Layer::TopCopper,
            hull: Hull::capsule(Point::new(0, 0), Point::new(200, 0), 2),
        });
        let c = analyze_items(&items);
        assert!(c.unrouted.is_empty());
        assert!(c.ratsnest.is_empty());
    }

    #[test]
    fn different_layers_do_not_connect_without_via() {
        let mut b = pad(2, 100, "A");
        b.layer = Layer::BottomCopper;
        let items = vec![
            pad(1, 0, "A"),
            b,
            CopperItem {
                what: CopperRef::Track(Id(9)),
                net: "A".into(),
                layer: Layer::TopCopper,
                hull: Hull::capsule(Point::new(0, 0), Point::new(100, 0), 2),
            },
        ];
        assert_eq!(analyze_items(&items).unrouted.get("A"), Some(&1));
    }
}
