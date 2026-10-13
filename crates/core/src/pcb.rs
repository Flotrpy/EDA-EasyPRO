//! PCB documents: placed footprints, tracks, vias, pours and graphics.

use crate::geom::{arc_points, circle_polygon, Point, Rect, Transform};
use crate::id::Id;
use crate::layer::{Layer, Side};
use crate::library::{Footprint, Hull, Library, Pad};
use crate::pour::Region;
use crate::shape::Shape;
use crate::units::mm;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbComponent {
    pub id: Id,
    /// The schematic component this footprint was created from.
    #[serde(default)]
    pub link: Option<Id>,
    pub designator: String,
    pub value: String,
    pub device: String,
    pub footprint: String,
    pub pos: Point,
    #[serde(default)]
    pub rotation: f64,
    #[serde(default)]
    pub side: Side,
    #[serde(default)]
    pub locked: bool,
    /// Pad number -> net name.
    #[serde(default)]
    pub pad_nets: BTreeMap<String, String>,
    /// Silkscreen designator position relative to `pos` (unrotated).
    #[serde(default)]
    pub designator_offset: Option<Point>,
    #[serde(default = "default_true")]
    pub show_designator: bool,
}

fn default_true() -> bool {
    true
}

impl PcbComponent {
    pub fn transform(&self) -> Transform {
        Transform::new(self.pos, self.rotation, self.side == Side::Bottom)
    }

    pub fn net_of(&self, pad: &str) -> &str {
        self.pad_nets.get(pad).map(|s| s.as_str()).unwrap_or("")
    }

    /// Map a footprint layer to the board layer for this placement.
    pub fn board_layer(&self, l: Layer) -> Layer {
        if self.side == Side::Bottom { l.flipped() } else { l }
    }

    pub fn bbox(&self, fp: &Footprint) -> Rect {
        let tf = self.transform();
        Rect::from_points(fp.bbox().corners().iter().map(|c| tf.apply(*c)))
    }

    pub fn designator_pos(&self, fp: &Footprint) -> Point {
        let off = self.designator_offset.unwrap_or_else(|| {
            let b = fp.bbox();
            Point::new(0, if b.is_empty() { mm(1.5) } else { b.max.y + mm(0.6) })
        });
        self.pos + crate::geom::rotate(off, self.rotation)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: Id,
    pub layer: Layer,
    pub a: Point,
    pub b: Point,
    pub width: i64,
    #[serde(default)]
    pub net: String,
}

impl Track {
    pub fn hull(&self) -> Hull {
        Hull::capsule(self.a, self.b, self.width / 2)
    }

    pub fn length(&self) -> f64 {
        self.a.dist(self.b)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Via {
    pub id: Id,
    pub pos: Point,
    pub diameter: i64,
    pub drill: i64,
    #[serde(default)]
    pub net: String,
}

impl Via {
    pub fn hull(&self) -> Hull {
        Hull::circle(self.pos, self.diameter / 2)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pour {
    pub id: Id,
    pub layer: Layer,
    pub net: String,
    pub outline: Vec<Point>,
    /// Clearance override; `None` uses the board rule.
    #[serde(default)]
    pub clearance: Option<i64>,
    #[serde(default = "default_true")]
    pub remove_islands: bool,
    /// Computed fill, rebuilt with "Rebuild copper areas".
    #[serde(default)]
    pub filled: Vec<Region>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbGraphic {
    pub id: Id,
    pub layer: Layer,
    pub shape: Shape,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetClass {
    pub name: String,
    pub clearance: i64,
    pub track_width: i64,
    pub via_diameter: i64,
    pub via_drill: i64,
    #[serde(default)]
    pub nets: Vec<String>,
}

/// Board design rules. Defaults match the capabilities of common low-cost
/// fabs (6 mil / 0.15 mm class, 0.3 mm drills).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DesignRules {
    pub clearance: i64,
    pub track_width: i64,
    pub min_track_width: i64,
    pub via_diameter: i64,
    pub via_drill: i64,
    pub min_drill: i64,
    pub min_annular_ring: i64,
    pub edge_clearance: i64,
    pub mask_expansion: i64,
    pub silk_width: i64,
    #[serde(default)]
    pub net_classes: Vec<NetClass>,
}

impl Default for DesignRules {
    fn default() -> Self {
        DesignRules {
            clearance: mm(0.2),
            track_width: mm(0.25),
            min_track_width: mm(0.15),
            via_diameter: mm(0.6),
            via_drill: mm(0.3),
            min_drill: mm(0.3),
            min_annular_ring: mm(0.13),
            edge_clearance: mm(0.3),
            mask_expansion: mm(0.05),
            silk_width: mm(0.15),
            net_classes: Vec::new(),
        }
    }
}

impl DesignRules {
    pub fn class_for(&self, net: &str) -> Option<&NetClass> {
        self.net_classes.iter().find(|c| c.nets.iter().any(|n| n == net))
    }

    pub fn clearance_for(&self, a: &str, b: &str) -> i64 {
        let ca = self.class_for(a).map(|c| c.clearance).unwrap_or(self.clearance);
        let cb = self.class_for(b).map(|c| c.clearance).unwrap_or(self.clearance);
        ca.max(cb)
    }

    pub fn track_width_for(&self, net: &str) -> i64 {
        self.class_for(net).map(|c| c.track_width).unwrap_or(self.track_width)
    }

    pub fn via_for(&self, net: &str) -> (i64, i64) {
        self.class_for(net)
            .map(|c| (c.via_diameter, c.via_drill))
            .unwrap_or((self.via_diameter, self.via_drill))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pcb {
    pub copper_layers: u8,
    /// Finished board thickness.
    pub thickness: i64,
    #[serde(default)]
    pub components: Vec<PcbComponent>,
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub vias: Vec<Via>,
    #[serde(default)]
    pub pours: Vec<Pour>,
    #[serde(default)]
    pub graphics: Vec<PcbGraphic>,
    #[serde(default)]
    pub rules: DesignRules,
    /// Solder mask colour for the 3D view and fab notes.
    #[serde(default = "default_mask")]
    pub mask_color: u32,
}

fn default_mask() -> u32 {
    0x1f6b3a
}

impl Default for Pcb {
    fn default() -> Self {
        Pcb {
            copper_layers: 2,
            thickness: mm(1.6),
            components: vec![],
            tracks: vec![],
            vias: vec![],
            pours: vec![],
            graphics: vec![],
            rules: DesignRules::default(),
            mask_color: default_mask(),
        }
    }
}

/// What a copper item is, for highlighting and DRC messages.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CopperRef {
    Pad { comp: Id, pad: String },
    Track(Id),
    Via(Id),
    Pour(Id, usize),
}

#[derive(Clone, Debug)]
pub struct CopperItem {
    pub what: CopperRef,
    pub net: String,
    pub layer: Layer,
    pub hull: Hull,
}

impl Pcb {
    pub fn component(&self, id: Id) -> Option<&PcbComponent> {
        self.components.iter().find(|c| c.id == id)
    }

    pub fn component_mut(&mut self, id: Id) -> Option<&mut PcbComponent> {
        self.components.iter_mut().find(|c| c.id == id)
    }

    pub fn remove(&mut self, id: Id) -> bool {
        let n = self.object_count();
        self.components.retain(|o| o.id != id);
        self.tracks.retain(|o| o.id != id);
        self.vias.retain(|o| o.id != id);
        self.pours.retain(|o| o.id != id);
        self.graphics.retain(|o| o.id != id);
        n != self.object_count()
    }

    pub fn object_count(&self) -> usize {
        self.components.len() + self.tracks.len() + self.vias.len() + self.pours.len() + self.graphics.len()
    }

    pub fn max_id(&self) -> Id {
        self.components.iter().map(|o| o.id)
            .chain(self.tracks.iter().map(|o| o.id))
            .chain(self.vias.iter().map(|o| o.id))
            .chain(self.pours.iter().map(|o| o.id))
            .chain(self.graphics.iter().map(|o| o.id))
            .max()
            .unwrap_or_default()
    }

    pub fn copper_stack(&self) -> Vec<Layer> {
        Layer::copper_stack(self.copper_layers)
    }

    /// Every pad on the board with its world hull and net.
    pub fn pads<'a>(&'a self, lib: &'a Library) -> impl Iterator<Item = (&'a PcbComponent, &'a Pad, Hull)> + 'a {
        self.components.iter().flat_map(move |c| {
            let tf = c.transform();
            lib.footprints
                .get(&c.footprint)
                .into_iter()
                .flat_map(move |fp| fp.pads.iter().map(move |p| (c, p, p.hull(&tf))))
        })
    }

    /// Flattened copper geometry for connectivity and DRC. Through-hole
    /// pads and vias are reported once with layer [`Layer::Multi`].
    pub fn copper_items(&self, lib: &Library) -> Vec<CopperItem> {
        let mut out = Vec::new();
        for (c, pad, hull) in self.pads(lib) {
            let layer = match pad.kind {
                crate::library::PadKind::Smd => c.board_layer(Layer::TopCopper),
                crate::library::PadKind::Tht { .. } => Layer::Multi,
                crate::library::PadKind::Npth { .. } => continue,
            };
            out.push(CopperItem {
                what: CopperRef::Pad { comp: c.id, pad: pad.number.clone() },
                net: c.net_of(&pad.number).to_string(),
                layer,
                hull,
            });
        }
        for t in &self.tracks {
            out.push(CopperItem { what: CopperRef::Track(t.id), net: t.net.clone(), layer: t.layer, hull: t.hull() });
        }
        for v in &self.vias {
            out.push(CopperItem { what: CopperRef::Via(v.id), net: v.net.clone(), layer: Layer::Multi, hull: v.hull() });
        }
        for p in &self.pours {
            for (i, r) in p.filled.iter().enumerate() {
                out.push(CopperItem {
                    what: CopperRef::Pour(p.id, i),
                    net: p.net.clone(),
                    layer: p.layer,
                    hull: Hull::polygon(r.outer.clone()),
                });
            }
        }
        out
    }

    /// The board shape, from outline-layer graphics. Closed shapes are used
    /// directly; otherwise line and arc segments are chained end to end.
    pub fn outline(&self) -> Option<Vec<Point>> {
        let shapes: Vec<&Shape> = self.graphics.iter().filter(|g| g.layer == Layer::Outline).map(|g| &g.shape).collect();
        for s in &shapes {
            match s {
                Shape::Rect { a, b, .. } => return Some(Rect::new(*a, *b).corners().to_vec()),
                Shape::Polygon { points, .. } if points.len() >= 3 => return Some(points.clone()),
                Shape::Circle { center, radius, .. } => return Some(circle_polygon(*center, *radius, 64)),
                _ => {}
            }
        }
        let mut segs: Vec<Vec<Point>> = shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Line { a, b, .. } => Some(vec![*a, *b]),
                Shape::Polyline { points, .. } => Some(points.clone()),
                Shape::Arc { center, radius, start, sweep, .. } => Some(arc_points(*center, *radius, *start, *sweep, 6.0)),
                _ => None,
            })
            .collect();
        let mut path = segs.pop()?;
        let tol = mm(0.01) as f64;
        while !segs.is_empty() {
            let end = *path.last()?;
            let idx = segs.iter().position(|s| s[0].dist(end) <= tol || s.last().unwrap().dist(end) <= tol)?;
            let mut s = segs.remove(idx);
            if s[0].dist(end) > tol {
                s.reverse();
            }
            path.extend(s.into_iter().skip(1));
        }
        if path.len() > 3 && path[0].dist(*path.last()?) <= tol {
            path.pop();
            Some(path)
        } else {
            None
        }
    }

    pub fn bbox(&self, lib: &Library) -> Rect {
        let mut r = Rect::EMPTY;
        if let Some(o) = self.outline() {
            r = Rect::from_points(o);
        }
        for c in &self.components {
            if let Some(fp) = lib.footprints.get(&c.footprint) {
                r = r.union(c.bbox(fp));
            }
        }
        for t in &self.tracks {
            r = r.union(t.hull().bbox());
        }
        for g in &self.graphics {
            r = r.union(g.shape.bbox());
        }
        r
    }

    /// All distinct net names used on the board, sorted.
    pub fn nets(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .components
            .iter()
            .flat_map(|c| c.pad_nets.values().cloned())
            .chain(self.tracks.iter().map(|t| t.net.clone()))
            .chain(self.vias.iter().map(|t| t.net.clone()))
            .filter(|n| !n.is_empty())
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// Recompute every copper pour.
    pub fn rebuild_pours(&mut self, lib: &Library) {
        let pours = std::mem::take(&mut self.pours);
        let mut rebuilt = Vec::with_capacity(pours.len());
        let items = self.copper_items(lib);
        let board = self.outline();
        for mut p in pours {
            let clearance = p.clearance.unwrap_or(self.rules.clearance);
            let mut obstacles = Vec::new();
            let mut keep = Vec::new();
            for it in items.iter().filter(|it| it.layer.shares_copper(p.layer)) {
                if !p.net.is_empty() && it.net == p.net {
                    keep.push(it.hull.clone());
                } else {
                    obstacles.push(it.hull.clone());
                }
            }
            // Keep copper away from the board edge and from bare holes.
            if let Some(b) = &board {
                let n = b.len();
                for i in 0..n {
                    obstacles.push(Hull::capsule(b[i], b[(i + 1) % n], self.rules.edge_clearance - clearance));
                }
            }
            for (c, pad, _) in self.pads(lib) {
                if let crate::library::PadKind::Npth { drill } = pad.kind {
                    obstacles.push(Hull::circle(c.transform().apply(pad.pos), drill / 2));
                }
            }
            let outline = match &board {
                Some(b) => clip_to(&p.outline, b),
                None => p.outline.clone(),
            };
            p.filled = crate::pour::fill(&outline, &obstacles, clearance, &keep, p.remove_islands && !keep.is_empty(), (mm(0.3) * mm(0.3)) as f64);
            rebuilt.push(p);
        }
        self.pours = rebuilt;
    }
}

/// Intersect a pour outline with the board outline (largest piece).
fn clip_to(poly: &[Point], board: &[Point]) -> Vec<Point> {
    use i_overlay::core::fill_rule::FillRule;
    use i_overlay::core::overlay_rule::OverlayRule;
    use i_overlay::float::overlay::FloatOverlay;
    let f = |p: &[Point]| vec![p.iter().map(|q| [q.x as f64, q.y as f64]).collect::<Vec<_>>()];
    let res = FloatOverlay::with_subj_and_clip(&f(poly), &f(board)).overlay(OverlayRule::Intersect, FillRule::NonZero);
    res.into_iter()
        .filter_map(|s| s.into_iter().next())
        .map(|c| c.iter().map(|p| Point::from_f(p[0], p[1])).collect::<Vec<_>>())
        .max_by(|a, b| crate::geom::polygon_area(a).abs().total_cmp(&crate::geom::polygon_area(b).abs()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(id: u64, a: (f64, f64), b: (f64, f64)) -> PcbGraphic {
        PcbGraphic {
            id: Id(id),
            layer: Layer::Outline,
            shape: Shape::line(Point::new(mm(a.0), mm(a.1)), Point::new(mm(b.0), mm(b.1)), mm(0.1)),
        }
    }

    #[test]
    fn outline_chains_segments_in_any_order() {
        let mut pcb = Pcb::default();
        pcb.graphics.push(line(1, (0.0, 0.0), (10.0, 0.0)));
        pcb.graphics.push(line(2, (10.0, 10.0), (0.0, 10.0)));
        pcb.graphics.push(line(3, (10.0, 10.0), (10.0, 0.0)));
        pcb.graphics.push(line(4, (0.0, 10.0), (0.0, 0.0)));
        let o = pcb.outline().expect("closed outline");
        assert_eq!(o.len(), 4);
    }

    #[test]
    fn open_outline_is_rejected() {
        let mut pcb = Pcb::default();
        pcb.graphics.push(line(1, (0.0, 0.0), (10.0, 0.0)));
        pcb.graphics.push(line(2, (10.0, 0.0), (10.0, 10.0)));
        assert!(pcb.outline().is_none());
    }

    #[test]
    fn net_class_overrides() {
        let mut r = DesignRules::default();
        r.net_classes.push(NetClass {
            name: "Power".into(),
            clearance: mm(0.3),
            track_width: mm(0.8),
            via_diameter: mm(1.0),
            via_drill: mm(0.5),
            nets: vec!["VCC".into()],
        });
        assert_eq!(r.track_width_for("VCC"), mm(0.8));
        assert_eq!(r.track_width_for("SDA"), r.track_width);
        assert_eq!(r.clearance_for("SDA", "VCC"), mm(0.3));
    }
}
