//! Schematic connectivity: which pins are joined by wires, labels and ports.

use crate::geom::{point_on_segment, Point, Rect};
use crate::id::Id;
use crate::library::{Library, PinKind};
use crate::schematic::Schematic;
use rstar::primitives::{GeomWithData, Rectangle};
use rstar::RTree;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetNode {
    pub comp: Id,
    pub designator: String,
    pub pin: String,
    pub pin_name: String,
    pub kind: PinKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Net {
    pub name: String,
    /// True when the name came from a label or port rather than generated.
    pub named: bool,
    pub nodes: Vec<NetNode>,
    /// Every label/port name attached (more than one is an ERC warning).
    pub aliases: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Connectivity {
    pub nets: Vec<Net>,
    /// (component, pin number) -> index into `nets`.
    pub pin_net: HashMap<(Id, String), usize>,
    /// Wire id -> index into `nets` (wires without pins are absent).
    pub wire_net: HashMap<Id, usize>,
    /// Junction dots per sheet.
    pub junctions: Vec<Vec<Point>>,
    /// Pins that touch nothing at all.
    pub unconnected: Vec<(usize, Id, String, Point)>,
}

impl Connectivity {
    pub fn net_of_pin(&self, comp: Id, pin: &str) -> Option<&Net> {
        self.pin_net.get(&(comp, pin.to_string())).map(|&i| &self.nets[i])
    }

    pub fn net_named(&self, name: &str) -> Option<&Net> {
        self.nets.iter().find(|n| n.name == name)
    }
}

struct Dsu(Vec<usize>);

impl Dsu {
    fn new(n: usize) -> Self {
        Dsu((0..n).collect())
    }
    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.0[r] != r {
            r = self.0[r];
        }
        let mut c = x;
        while self.0[c] != r {
            let next = self.0[c];
            self.0[c] = r;
            c = next;
        }
        r
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.0[ra.max(rb)] = ra.min(rb);
        }
    }
}

enum Node {
    Pin { sheet: usize, comp: Id, designator: String, number: String, name: String, kind: PinKind, pos: Point },
    Wire { id: Id },
    Name { name: String, power: bool },
}

type SegIndex = RTree<GeomWithData<Rectangle<[i64; 2]>, (usize, Point, Point)>>;

fn seg_index(segs: &[(usize, Point, Point)]) -> SegIndex {
    RTree::bulk_load(
        segs.iter()
            .map(|&(node, a, b)| {
                let r = Rect::new(a, b);
                GeomWithData::new(Rectangle::from_corners([r.min.x, r.min.y], [r.max.x, r.max.y]), (node, a, b))
            })
            .collect(),
    )
}

fn segments_at(index: &SegIndex, p: Point) -> impl Iterator<Item = &(usize, Point, Point)> {
    index
        .locate_all_at_point(&[p.x, p.y])
        .map(|g| &g.data)
        .filter(move |(_, a, b)| point_on_segment(p, *a, *b))
}

/// Compute nets for a whole multi-sheet schematic.
pub fn connectivity(sch: &Schematic, lib: &Library) -> Connectivity {
    let mut nodes: Vec<Node> = Vec::new();
    let mut joins: Vec<(usize, usize)> = Vec::new();
    let mut junctions = Vec::new();

    for (si, sheet) in sch.sheets.iter().enumerate() {
        let mut segs = Vec::new();
        let mut wire_pts: Vec<(usize, Point)> = Vec::new();
        for w in &sheet.wires {
            let n = nodes.len();
            nodes.push(Node::Wire { id: w.id });
            for (a, b) in w.segments() {
                segs.push((n, a, b));
            }
            for p in &w.points {
                wire_pts.push((n, *p));
            }
        }
        let index = seg_index(&segs);

        // Wire to wire: any vertex touching another wire's segment.
        for &(n, p) in &wire_pts {
            for &(m, _, _) in segments_at(&index, p) {
                if m != n {
                    joins.push((n, m));
                }
            }
        }

        // Pins.
        let mut pins_here: HashMap<Point, Vec<usize>> = HashMap::new();
        for c in &sheet.components {
            let Some(dev) = lib.devices.get(&c.device) else { continue };
            let Some(sym) = lib.symbols.get(&dev.symbol) else { continue };
            let tf = c.transform();
            for pin in &sym.pins {
                let pos = tf.apply(pin.pos);
                let n = nodes.len();
                nodes.push(Node::Pin {
                    sheet: si,
                    comp: c.id,
                    designator: c.designator.clone(),
                    number: pin.number.clone(),
                    name: pin.name.clone(),
                    kind: pin.kind,
                    pos,
                });
                for &(m, _, _) in segments_at(&index, pos) {
                    joins.push((n, m));
                }
                pins_here.entry(pos).or_default().push(n);
            }
        }
        for group in pins_here.values() {
            for w in group.windows(2) {
                joins.push((w[0], w[1]));
            }
        }

        // Labels and ports attach to wires or pins under their anchor.
        let attach = |name: &str, pos: Point, power: bool, nodes: &mut Vec<Node>, joins: &mut Vec<(usize, usize)>| {
            let n = nodes.len();
            nodes.push(Node::Name { name: name.to_string(), power });
            for &(m, _, _) in segments_at(&index, pos) {
                joins.push((n, m));
            }
            if let Some(ps) = pins_here.get(&pos) {
                for &m in ps {
                    joins.push((n, m));
                }
            }
        };
        for l in &sheet.labels {
            attach(&l.name, l.pos, false, &mut nodes, &mut joins);
        }
        for p in &sheet.ports {
            attach(&p.name, p.pos, true, &mut nodes, &mut joins);
        }

        // Junction dots: three or more connections meeting at a point.
        let mut counts: HashMap<Point, usize> = HashMap::new();
        for w in &sheet.wires {
            if let (Some(a), Some(b)) = (w.points.first(), w.points.last()) {
                *counts.entry(*a).or_default() += 1;
                *counts.entry(*b).or_default() += 1;
            }
        }
        for (p, ps) in &pins_here {
            *counts.entry(*p).or_default() += ps.len();
        }
        let mut dots = Vec::new();
        for (&p, &n) in &counts {
            let through = segments_at(&index, p).filter(|(_, a, b)| *a != p && *b != p).count();
            // Interior polyline vertices count as two connections.
            let bends = sheet
                .wires
                .iter()
                .filter(|w| w.points.len() > 2 && w.points[1..w.points.len() - 1].contains(&p))
                .count();
            if n + 2 * through + 2 * bends >= 3 {
                dots.push(p);
            }
        }
        dots.sort();
        junctions.push(dots);
    }

    // Same names join globally.
    let mut by_name: HashMap<String, usize> = HashMap::new();
    for (i, n) in nodes.iter().enumerate() {
        if let Node::Name { name, .. } = n {
            if let Some(&j) = by_name.get(name) {
                joins.push((i, j));
            } else {
                by_name.insert(name.clone(), i);
            }
        }
    }

    let mut dsu = Dsu::new(nodes.len());
    for (a, b) in joins {
        dsu.union(a, b);
    }

    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..nodes.len() {
        groups.entry(dsu.find(i)).or_default().push(i);
    }

    let mut out = Connectivity { junctions, ..Default::default() };
    for members in groups.values() {
        let mut pins = Vec::new();
        let mut names: Vec<(bool, String)> = Vec::new();
        let mut wires = Vec::new();
        for &i in members {
            match &nodes[i] {
                Node::Pin { designator, number, name, kind, comp, .. } => pins.push(NetNode {
                    comp: *comp,
                    designator: designator.clone(),
                    pin: number.clone(),
                    pin_name: name.clone(),
                    kind: *kind,
                }),
                Node::Wire { id } => wires.push(*id),
                Node::Name { name, power } => names.push((*power, name.clone())),
            }
        }
        if pins.is_empty() {
            continue;
        }
        if pins.len() == 1 && names.is_empty() && wires.is_empty() {
            if let Node::Pin { sheet, comp, number, pos, .. } = &nodes[members[0]] {
                out.unconnected.push((*sheet, *comp, number.clone(), *pos));
            }
            continue;
        }
        // Power ports win over labels; ties break alphabetically.
        names.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        names.dedup_by(|a, b| a.1 == b.1);
        pins.sort_by(|a, b| natural_cmp(&a.designator, &b.designator).then(natural_cmp(&a.pin, &b.pin)));
        let (name, named) = match names.first() {
            Some((_, n)) => (n.clone(), true),
            None => (format!("Net-({}-{})", pins[0].designator, pins[0].pin), false),
        };
        let idx = out.nets.len();
        for p in &pins {
            out.pin_net.insert((p.comp, p.pin.clone()), idx);
        }
        for w in wires {
            out.wire_net.insert(w, idx);
        }
        out.nets.push(Net { name, named, nodes: pins, aliases: names.into_iter().map(|n| n.1).collect() });
    }
    out
}

/// Compare strings so that `R2 < R10` and `1 < 2 < 10`.
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn split(s: &str) -> (String, u64, String) {
        let pre: String = s.chars().take_while(|c| !c.is_ascii_digit()).collect();
        let rest = &s[pre.len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let tail = rest[digits.len()..].to_string();
        (pre, digits.parse().unwrap_or(0), tail)
    }
    let (pa, na, ta) = split(a);
    let (pb, nb, tb) = split(b);
    pa.cmp(&pb).then(na.cmp(&nb)).then(ta.cmp(&tb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{Device, Pin, Symbol};
    use crate::schematic::{Component, NetLabel, NetPort, PortStyle, Sheet, Wire};

    fn lib() -> Library {
        let mut l = Library::default();
        l.add_symbol(Symbol {
            id: "res".into(),
            name: "Resistor".into(),
            pins: vec![
                Pin { number: "1".into(), name: "1".into(), pos: Point::new(-20, 0), angle: 0.0, length: 10, kind: PinKind::Passive, hide_name: true },
                Pin { number: "2".into(), name: "2".into(), pos: Point::new(20, 0), angle: 180.0, length: 10, kind: PinKind::Passive, hide_name: true },
            ],
            graphics: vec![],
        });
        l.add_device(Device { id: "R".into(), name: "R".into(), prefix: "R".into(), symbol: "res".into(), ..Default::default() });
        l
    }

    fn comp(id: u64, des: &str, x: i64, y: i64) -> Component {
        Component {
            id: Id(id),
            device: "R".into(),
            designator: des.into(),
            value: "10k".into(),
            pos: Point::new(x, y),
            rotation: 0.0,
            mirror: false,
            attributes: Default::default(),
            exclude_from_bom: false,
            designator_offset: None,
            value_offset: None,
        }
    }

    fn wire(id: u64, pts: &[(i64, i64)]) -> Wire {
        Wire { id: Id(id), points: pts.iter().map(|&(x, y)| Point::new(x, y)).collect() }
    }

    #[test]
    fn wire_connects_two_resistors() {
        let mut s = Sheet::default();
        s.components.push(comp(1, "R1", 0, 0));
        s.components.push(comp(2, "R2", 100, 0));
        s.wires.push(wire(3, &[(20, 0), (80, 0)]));
        let sch = Schematic { sheets: vec![s] };
        let c = connectivity(&sch, &lib());
        assert_eq!(c.nets.len(), 1);
        assert_eq!(c.nets[0].name, "Net-(R1-2)");
        assert_eq!(c.nets[0].nodes.len(), 2);
        assert_eq!(c.unconnected.len(), 2);
    }

    #[test]
    fn labels_join_across_sheets_and_name_the_net() {
        let mut a = Sheet::default();
        a.components.push(comp(1, "R1", 0, 0));
        a.wires.push(wire(2, &[(20, 0), (40, 0)]));
        a.labels.push(NetLabel { id: Id(3), name: "SDA".into(), pos: Point::new(40, 0), rotation: 0.0 });
        let mut b = Sheet::default();
        b.components.push(comp(4, "R2", 0, 0));
        b.ports.push(NetPort { id: Id(5), name: "SDA".into(), pos: Point::new(-20, 0), style: PortStyle::Port, rotation: 0.0 });
        let sch = Schematic { sheets: vec![a, b] };
        let c = connectivity(&sch, &lib());
        let sda = c.net_named("SDA").expect("SDA net");
        assert_eq!(sda.nodes.len(), 2);
        assert!(sda.named);
    }

    #[test]
    fn t_junction_connects_and_gets_a_dot() {
        let mut s = Sheet::default();
        s.components.push(comp(1, "R1", 0, 0));
        s.components.push(comp(2, "R2", 0, 100));
        s.wires.push(wire(3, &[(20, 0), (60, 0)]));
        s.wires.push(wire(4, &[(20, 100), (40, 100), (40, 0)]));
        let sch = Schematic { sheets: vec![s] };
        let c = connectivity(&sch, &lib());
        assert_eq!(c.nets.len(), 1);
        assert_eq!(c.junctions[0], vec![Point::new(40, 0)]);
    }

    #[test]
    fn crossing_wires_do_not_connect() {
        let mut s = Sheet::default();
        s.components.push(comp(1, "R1", 0, 0));
        s.components.push(comp(2, "R2", 200, 0));
        s.wires.push(wire(3, &[(20, 0), (60, 0)]));
        s.wires.push(wire(4, &[(40, -50), (40, 50)]));
        let sch = Schematic { sheets: vec![s] };
        let c = connectivity(&sch, &lib());
        assert_eq!(c.nets.len(), 1);
        assert_eq!(c.nets[0].nodes.len(), 1);
        assert!(!c.wire_net.contains_key(&Id(4)));
        assert!(c.junctions[0].is_empty());
    }

    #[test]
    fn natural_ordering() {
        let mut v = vec!["R10", "R2", "C1", "R1"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["C1", "R1", "R2", "R10"]);
    }
}
