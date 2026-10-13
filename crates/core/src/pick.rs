//! Hit testing for the editors.

use crate::font;
use crate::geom::{point_segment_dist, Point, Rect};
use crate::id::Id;
use crate::layer::Layer;
use crate::library::{Library, PadKind};
use crate::pcb::Pcb;
use crate::scene::LABEL_TEXT;
use crate::schematic::Sheet;

/// What kind of object an id refers to (for property panels and tools).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchKind {
    Component,
    Wire,
    Label,
    Port,
    NoConnect,
    Annotation,
}

pub fn sch_kind(sheet: &Sheet, id: Id) -> Option<SchKind> {
    if sheet.components.iter().any(|o| o.id == id) {
        Some(SchKind::Component)
    } else if sheet.wires.iter().any(|o| o.id == id) {
        Some(SchKind::Wire)
    } else if sheet.labels.iter().any(|o| o.id == id) {
        Some(SchKind::Label)
    } else if sheet.ports.iter().any(|o| o.id == id) {
        Some(SchKind::Port)
    } else if sheet.no_connects.iter().any(|o| o.id == id) {
        Some(SchKind::NoConnect)
    } else if sheet.annotations.iter().any(|o| o.id == id) {
        Some(SchKind::Annotation)
    } else {
        None
    }
}

fn label_box(pos: Point, name: &str) -> Rect {
    Rect::new(
        pos,
        pos + Point::new(
            font::text_width(name, LABEL_TEXT) + 300_000,
            LABEL_TEXT + 500_000,
        ),
    )
}

/// Bounding box of a schematic object.
pub fn sch_bbox(sheet: &Sheet, lib: &Library, id: Id) -> Option<Rect> {
    if let Some(c) = sheet.component(id) {
        return Some(c.bbox(lib));
    }
    if let Some(w) = sheet.wires.iter().find(|o| o.id == id) {
        return Some(Rect::from_points(w.points.iter().copied()));
    }
    if let Some(l) = sheet.labels.iter().find(|o| o.id == id) {
        return Some(label_box(l.pos, &l.name));
    }
    if let Some(p) = sheet.ports.iter().find(|o| o.id == id) {
        return Some(crate::scene::port_bbox(p));
    }
    if let Some(n) = sheet.no_connects.iter().find(|o| o.id == id) {
        return Some(Rect::around(n.pos, 800_000, 800_000));
    }
    sheet
        .annotations
        .iter()
        .find(|o| o.id == id)
        .map(|a| a.shape.bbox())
}

/// Topmost schematic object under `p` within `tol` nm.
pub fn sch_hit(sheet: &Sheet, lib: &Library, p: Point, tol: i64) -> Option<Id> {
    let tol_f = tol as f64;
    for l in sheet.labels.iter().rev() {
        if label_box(l.pos, &l.name).inflate(tol / 2).contains(p) {
            return Some(l.id);
        }
    }
    for port in sheet.ports.iter().rev() {
        if sch_bbox(sheet, lib, port.id).is_some_and(|b| b.contains(p)) {
            return Some(port.id);
        }
    }
    for n in sheet.no_connects.iter().rev() {
        if n.pos.dist(p) <= 800_000.0 + tol_f {
            return Some(n.id);
        }
    }
    // Wires before component bodies so a wire crossing a body stays pickable.
    let mut best: Option<(f64, Id)> = None;
    for w in &sheet.wires {
        let d = w
            .segments()
            .map(|(a, b)| point_segment_dist(p, a, b))
            .fold(f64::INFINITY, f64::min);
        if d <= tol_f && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, w.id));
        }
    }
    if let Some((_, id)) = best {
        return Some(id);
    }
    for c in sheet.components.iter().rev() {
        if c.bbox(lib).inflate(tol / 2).contains(p) {
            return Some(c.id);
        }
    }
    for a in sheet.annotations.iter().rev() {
        if a.shape.distance_to(p) <= tol_f {
            return Some(a.id);
        }
    }
    None
}

/// Objects fully inside `r` (or touching it when `crossing`).
pub fn sch_in_rect(sheet: &Sheet, lib: &Library, r: Rect, crossing: bool) -> Vec<Id> {
    let all = sheet
        .components
        .iter()
        .map(|o| o.id)
        .chain(sheet.wires.iter().map(|o| o.id))
        .chain(sheet.labels.iter().map(|o| o.id))
        .chain(sheet.ports.iter().map(|o| o.id))
        .chain(sheet.no_connects.iter().map(|o| o.id))
        .chain(sheet.annotations.iter().map(|o| o.id));
    all.filter(|id| {
        sch_bbox(sheet, lib, *id).is_some_and(|b| {
            if crossing {
                r.intersects(&b)
            } else {
                r.contains_rect(&b)
            }
        })
    })
    .collect()
}

/// A pin connection point near `p`, if any (for wire snapping).
pub fn sch_pin_near(sheet: &Sheet, lib: &Library, p: Point, tol: i64) -> Option<Point> {
    let mut best: Option<(f64, Point)> = None;
    for c in &sheet.components {
        let Some(sym) = lib
            .devices
            .get(&c.device)
            .and_then(|d| lib.symbols.get(&d.symbol))
        else {
            continue;
        };
        for (_, q) in c.pin_positions(sym) {
            let d = q.dist(p);
            if d <= tol as f64 && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, q));
            }
        }
    }
    for w in &sheet.wires {
        for q in [w.points.first(), w.points.last()].into_iter().flatten() {
            let d = q.dist(p);
            if d <= tol as f64 && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, *q));
            }
        }
    }
    best.map(|(_, q)| q)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcbKind {
    Component,
    Track,
    Via,
    Pour,
    Graphic,
}

pub fn pcb_kind(pcb: &Pcb, id: Id) -> Option<PcbKind> {
    if pcb.components.iter().any(|o| o.id == id) {
        Some(PcbKind::Component)
    } else if pcb.tracks.iter().any(|o| o.id == id) {
        Some(PcbKind::Track)
    } else if pcb.vias.iter().any(|o| o.id == id) {
        Some(PcbKind::Via)
    } else if pcb.pours.iter().any(|o| o.id == id) {
        Some(PcbKind::Pour)
    } else if pcb.graphics.iter().any(|o| o.id == id) {
        Some(PcbKind::Graphic)
    } else {
        None
    }
}

pub fn pcb_bbox(pcb: &Pcb, lib: &Library, id: Id) -> Option<Rect> {
    if let Some(c) = pcb.component(id) {
        return lib.footprints.get(&c.footprint).map(|f| c.bbox(f));
    }
    if let Some(t) = pcb.tracks.iter().find(|o| o.id == id) {
        return Some(t.hull().bbox());
    }
    if let Some(v) = pcb.vias.iter().find(|o| o.id == id) {
        return Some(v.hull().bbox());
    }
    if let Some(p) = pcb.pours.iter().find(|o| o.id == id) {
        return Some(Rect::from_points(p.outline.iter().copied()));
    }
    pcb.graphics
        .iter()
        .find(|o| o.id == id)
        .map(|g| g.shape.bbox())
}

/// Topmost board object under `p`. Items on `active` win ties.
pub fn pcb_hit(
    pcb: &Pcb,
    lib: &Library,
    p: Point,
    tol: i64,
    active: Layer,
    visible: &dyn Fn(Layer) -> bool,
) -> Option<Id> {
    let tol_f = tol as f64;
    for v in pcb.vias.iter().rev() {
        if v.pos.dist(p) <= (v.diameter / 2) as f64 + tol_f {
            return Some(v.id);
        }
    }
    let mut tracks: Vec<_> = pcb.tracks.iter().filter(|t| visible(t.layer)).collect();
    tracks.sort_by_key(|t| t.layer != active);
    for t in tracks {
        if point_segment_dist(p, t.a, t.b) <= (t.width / 2) as f64 + tol_f {
            return Some(t.id);
        }
    }
    // Footprints: pads first, then the body box.
    for c in pcb.components.iter().rev() {
        let Some(fp) = lib.footprints.get(&c.footprint) else {
            continue;
        };
        let tf = c.transform();
        let pad_hit = fp.pads.iter().any(|pad| {
            let on = match pad.kind {
                PadKind::Smd => visible(c.board_layer(Layer::TopCopper)),
                _ => visible(Layer::Multi),
            };
            on && pad.hull(&tf).distance(&crate::library::Hull::circle(p, 0)) <= tol_f
        });
        if pad_hit || c.bbox(fp).contains(p) {
            return Some(c.id);
        }
    }
    for g in pcb.graphics.iter().rev().filter(|g| visible(g.layer)) {
        if g.shape.distance_to(p) <= tol_f.max(g.shape_width() as f64) {
            return Some(g.id);
        }
    }
    for pour in pcb.pours.iter().rev().filter(|po| visible(po.layer)) {
        let n = pour.outline.len();
        let on_edge = (0..n).any(|i| {
            point_segment_dist(p, pour.outline[i], pour.outline[(i + 1) % n]) <= tol_f * 2.0
        });
        if on_edge || pour.filled.iter().any(|r| r.contains(p)) {
            return Some(pour.id);
        }
    }
    None
}

pub fn pcb_in_rect(pcb: &Pcb, lib: &Library, r: Rect, crossing: bool) -> Vec<Id> {
    let ids = pcb
        .components
        .iter()
        .map(|o| o.id)
        .chain(pcb.tracks.iter().map(|o| o.id))
        .chain(pcb.vias.iter().map(|o| o.id))
        .chain(pcb.pours.iter().map(|o| o.id))
        .chain(pcb.graphics.iter().map(|o| o.id));
    ids.filter(|id| {
        pcb_bbox(pcb, lib, *id).is_some_and(|b| {
            if crossing {
                r.intersects(&b)
            } else {
                r.contains_rect(&b)
            }
        })
    })
    .collect()
}

impl crate::pcb::PcbGraphic {
    fn shape_width(&self) -> i64 {
        use crate::shape::Shape::*;
        match &self.shape {
            Line { width, .. }
            | Polyline { width, .. }
            | Rect { width, .. }
            | Circle { width, .. }
            | Arc { width, .. }
            | Polygon { width, .. } => *width,
            Text { size, .. } => *size / 2,
        }
    }
}

/// Pad or track end under `p` on `layer`: snapping point and its net.
pub fn pcb_anchor(
    pcb: &Pcb,
    lib: &Library,
    p: Point,
    tol: i64,
    layer: Layer,
) -> Option<(Point, String)> {
    let mut best: Option<(f64, Point, String)> = None;
    let mut consider = |q: Point, net: &str, d: f64| {
        if d <= tol as f64 && best.as_ref().is_none_or(|(bd, ..)| d < *bd) {
            best = Some((d, q, net.to_string()));
        }
    };
    for (c, pad, hull) in pcb.pads(lib) {
        if !pad.on_layer(layer, c.side) {
            continue;
        }
        let center = c.transform().apply(pad.pos);
        let d = hull.distance(&crate::library::Hull::circle(p, 0));
        if d <= tol as f64 {
            consider(center, c.net_of(&pad.number), center.dist(p) * 0.001);
        }
    }
    for t in pcb.tracks.iter().filter(|t| t.layer == layer) {
        for q in [t.a, t.b] {
            consider(q, &t.net, q.dist(p));
        }
    }
    for v in &pcb.vias {
        consider(v.pos, &v.net, v.pos.dist(p));
    }
    best.map(|(_, q, n)| (q, n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::examples::blinker;
    use crate::units::mm;

    #[test]
    fn picks_in_example() {
        let p = blinker();
        let b = &p.boards[0];
        let sheet = &b.schematic.sheets[0];
        let u1 = sheet
            .components
            .iter()
            .find(|c| c.designator == "U1")
            .unwrap();
        assert_eq!(sch_hit(sheet, &p.library, u1.pos, mm(0.5)), Some(u1.id));
        let w = &sheet.wires[0];
        let mid = Point::new(
            (w.points[0].x + w.points[1].x) / 2,
            (w.points[0].y + w.points[1].y) / 2,
        );
        assert_eq!(sch_hit(sheet, &p.library, mid, mm(0.3)), Some(w.id));
        assert!(
            sch_in_rect(sheet, &p.library, sheet.frame(), false).len() >= sheet.object_count() - 1
        );

        let pcb = &b.pcb;
        let pu1 = pcb
            .components
            .iter()
            .find(|c| c.designator == "U1")
            .unwrap();
        let all = |_l: Layer| true;
        assert_eq!(
            pcb_hit(pcb, &p.library, pu1.pos, mm(0.1), Layer::TopCopper, &all),
            Some(pu1.id)
        );
        let (snap, net) = pcb_anchor(
            pcb,
            &p.library,
            pu1.transform().apply(Point::new(mm(-2.7), mm(1.905))),
            mm(0.3),
            Layer::TopCopper,
        )
        .unwrap();
        assert_eq!(net, "GND");
        assert_eq!(snap, pu1.transform().apply(Point::new(mm(-2.7), mm(1.905))));
    }
}
