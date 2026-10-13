//! Editing operations shared by the schematic and PCB editors.

use crate::geom::{normalize_angle, rotate, Point, Rect, Transform};
use crate::id::{Id, IdGen};
use crate::layer::Side;
use crate::library::Library;
use crate::pcb::{Pcb, PcbComponent, PcbGraphic, Pour, Track, Via};
use crate::schematic::{
    Annotation, Component, NetLabel, NetPort, NoConnect, PortStyle, Sheet, Wire,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

fn rot_about(p: Point, c: Point, deg: f64) -> Point {
    rotate(p - c, deg) + c
}

fn mirror_about(p: Point, c: Point) -> Point {
    Point::new(2 * c.x - p.x, p.y)
}

/// Pin positions of the given components.
fn pins_of(sheet: &Sheet, lib: &Library, ids: &HashSet<Id>) -> Vec<Point> {
    sheet
        .components
        .iter()
        .filter(|c| ids.contains(&c.id))
        .filter_map(|c| {
            lib.devices
                .get(&c.device)
                .and_then(|d| lib.symbols.get(&d.symbol))
                .map(|s| c.pin_positions(s).map(|(_, p)| p).collect::<Vec<_>>())
        })
        .flatten()
        .collect()
}

/// Move schematic objects. Wire ends attached to moved pins follow, with
/// an extra bend so wires stay orthogonal.
pub fn sch_move(sheet: &mut Sheet, lib: &Library, ids: &[Id], d: Point) {
    if d == Point::ZERO {
        return;
    }
    let set: HashSet<Id> = ids.iter().copied().collect();
    let pins: HashSet<Point> = pins_of(sheet, lib, &set).into_iter().collect();
    for c in sheet.components.iter_mut().filter(|c| set.contains(&c.id)) {
        c.pos = c.pos + d;
    }
    for w in &mut sheet.wires {
        if set.contains(&w.id) {
            w.points.iter_mut().for_each(|p| *p = *p + d);
            continue;
        }
        let n = w.points.len();
        for end in [0, n - 1] {
            let e = w.points[end];
            if !pins.contains(&e) {
                continue;
            }
            let neighbor = if end == 0 {
                w.points[1]
            } else {
                w.points[n - 2]
            };
            let moved = e + d;
            let corner = if neighbor.y == e.y {
                Point::new(neighbor.x, moved.y)
            } else {
                Point::new(moved.x, neighbor.y)
            };
            if end == 0 {
                w.points[0] = moved;
                if corner != moved && corner != neighbor {
                    w.points.insert(1, corner);
                }
            } else {
                w.points[n - 1] = moved;
                if corner != moved && corner != neighbor {
                    w.points.insert(n - 1, corner);
                }
            }
            break;
        }
    }
    for l in sheet.labels.iter_mut().filter(|o| set.contains(&o.id)) {
        l.pos = l.pos + d;
    }
    for p in sheet.ports.iter_mut().filter(|o| set.contains(&o.id)) {
        p.pos = p.pos + d;
    }
    for n in sheet.no_connects.iter_mut().filter(|o| set.contains(&o.id)) {
        n.pos = n.pos + d;
    }
    for a in sheet.annotations.iter_mut().filter(|o| set.contains(&o.id)) {
        a.shape = a.shape.transformed(&Transform::translate(d));
    }
    sheet.tidy_wires();
}

/// Rotate schematic objects counter-clockwise by `deg` about `c`.
pub fn sch_rotate(sheet: &mut Sheet, ids: &[Id], c: Point, deg: f64) {
    let set: HashSet<Id> = ids.iter().copied().collect();
    for comp in sheet.components.iter_mut().filter(|o| set.contains(&o.id)) {
        comp.pos = rot_about(comp.pos, c, deg);
        comp.rotation = normalize_angle(comp.rotation + if comp.mirror { -deg } else { deg });
    }
    for w in sheet.wires.iter_mut().filter(|o| set.contains(&o.id)) {
        w.points.iter_mut().for_each(|p| *p = rot_about(*p, c, deg));
    }
    for l in sheet.labels.iter_mut().filter(|o| set.contains(&o.id)) {
        l.pos = rot_about(l.pos, c, deg);
        l.rotation = normalize_angle(l.rotation + deg);
        // Keep text readable.
        if l.rotation == 180.0 {
            l.rotation = 0.0;
        } else if l.rotation == 270.0 {
            l.rotation = 90.0;
        }
    }
    for p in sheet.ports.iter_mut().filter(|o| set.contains(&o.id)) {
        p.pos = rot_about(p.pos, c, deg);
        if p.style == PortStyle::Port {
            p.rotation = normalize_angle(p.rotation + deg);
        }
    }
    for n in sheet.no_connects.iter_mut().filter(|o| set.contains(&o.id)) {
        n.pos = rot_about(n.pos, c, deg);
    }
    let tf = Transform::new(c, deg, false);
    for a in sheet.annotations.iter_mut().filter(|o| set.contains(&o.id)) {
        a.shape = a
            .shape
            .transformed(&Transform::translate(-c))
            .transformed(&tf);
    }
}

/// Mirror schematic objects left-right about the vertical line through `c`.
pub fn sch_mirror(sheet: &mut Sheet, ids: &[Id], c: Point) {
    let set: HashSet<Id> = ids.iter().copied().collect();
    for comp in sheet.components.iter_mut().filter(|o| set.contains(&o.id)) {
        comp.pos = mirror_about(comp.pos, c);
        comp.mirror = !comp.mirror;
        comp.rotation = normalize_angle(-comp.rotation);
    }
    for w in sheet.wires.iter_mut().filter(|o| set.contains(&o.id)) {
        w.points.iter_mut().for_each(|p| *p = mirror_about(*p, c));
    }
    for l in sheet.labels.iter_mut().filter(|o| set.contains(&o.id)) {
        l.pos = mirror_about(l.pos, c);
    }
    for p in sheet.ports.iter_mut().filter(|o| set.contains(&o.id)) {
        p.pos = mirror_about(p.pos, c);
        if p.style == PortStyle::Port {
            p.rotation = normalize_angle(180.0 - p.rotation);
        }
    }
    for n in sheet.no_connects.iter_mut().filter(|o| set.contains(&o.id)) {
        n.pos = mirror_about(n.pos, c);
    }
}

/// Copied schematic objects (also the clipboard format).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SchFragment {
    pub components: Vec<Component>,
    pub wires: Vec<Wire>,
    pub labels: Vec<NetLabel>,
    pub ports: Vec<NetPort>,
    pub no_connects: Vec<NoConnect>,
    pub annotations: Vec<Annotation>,
}

impl SchFragment {
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
            && self.wires.is_empty()
            && self.labels.is_empty()
            && self.ports.is_empty()
            && self.no_connects.is_empty()
            && self.annotations.is_empty()
    }
}

pub fn sch_copy(sheet: &Sheet, ids: &[Id]) -> SchFragment {
    let set: HashSet<Id> = ids.iter().copied().collect();
    SchFragment {
        components: sheet
            .components
            .iter()
            .filter(|o| set.contains(&o.id))
            .cloned()
            .collect(),
        wires: sheet
            .wires
            .iter()
            .filter(|o| set.contains(&o.id))
            .cloned()
            .collect(),
        labels: sheet
            .labels
            .iter()
            .filter(|o| set.contains(&o.id))
            .cloned()
            .collect(),
        ports: sheet
            .ports
            .iter()
            .filter(|o| set.contains(&o.id))
            .cloned()
            .collect(),
        no_connects: sheet
            .no_connects
            .iter()
            .filter(|o| set.contains(&o.id))
            .cloned()
            .collect(),
        annotations: sheet
            .annotations
            .iter()
            .filter(|o| set.contains(&o.id))
            .cloned()
            .collect(),
    }
}

/// Paste with fresh ids, offset by `d`. Pasted parts get `?` designators so
/// annotation renumbers them. Returns the new ids.
pub fn sch_paste(sheet: &mut Sheet, frag: &SchFragment, d: Point, ids: &mut IdGen) -> Vec<Id> {
    let mut out = Vec::new();
    let mut fresh = |out: &mut Vec<Id>| {
        let id = ids.fresh();
        out.push(id);
        id
    };
    for c in &frag.components {
        let mut c = c.clone();
        c.id = fresh(&mut out);
        c.pos = c.pos + d;
        let prefix: String = c
            .designator
            .chars()
            .take_while(|ch| !ch.is_ascii_digit() && *ch != '?')
            .collect();
        c.designator = format!("{prefix}?");
        sheet.components.push(c);
    }
    for w in &frag.wires {
        sheet.wires.push(Wire {
            id: fresh(&mut out),
            points: w.points.iter().map(|p| *p + d).collect(),
        });
    }
    for l in &frag.labels {
        sheet.labels.push(NetLabel {
            id: fresh(&mut out),
            pos: l.pos + d,
            ..l.clone()
        });
    }
    for p in &frag.ports {
        sheet.ports.push(NetPort {
            id: fresh(&mut out),
            pos: p.pos + d,
            ..p.clone()
        });
    }
    for n in &frag.no_connects {
        sheet.no_connects.push(NoConnect {
            id: fresh(&mut out),
            pos: n.pos + d,
        });
    }
    for a in &frag.annotations {
        sheet.annotations.push(Annotation {
            id: fresh(&mut out),
            shape: a.shape.transformed(&Transform::translate(d)),
        });
    }
    out
}

pub fn frag_bbox(frag: &SchFragment, lib: &Library) -> Rect {
    let mut r = Rect::EMPTY;
    for c in &frag.components {
        r = r.union(c.bbox(lib));
    }
    for w in &frag.wires {
        r = r.union(Rect::from_points(w.points.iter().copied()));
    }
    for p in frag
        .labels
        .iter()
        .map(|l| l.pos)
        .chain(frag.ports.iter().map(|p| p.pos))
        .chain(frag.no_connects.iter().map(|n| n.pos))
    {
        r.include(p);
    }
    r
}

// ---------------------------------------------------------------- PCB

pub fn pcb_move(pcb: &mut Pcb, ids: &[Id], d: Point) -> usize {
    let set: HashSet<Id> = ids.iter().copied().collect();
    let mut moved = 0;
    for c in pcb
        .components
        .iter_mut()
        .filter(|o| set.contains(&o.id) && !o.locked)
    {
        c.pos = c.pos + d;
        moved += 1;
    }
    for t in pcb.tracks.iter_mut().filter(|o| set.contains(&o.id)) {
        t.a = t.a + d;
        t.b = t.b + d;
        moved += 1;
    }
    for v in pcb.vias.iter_mut().filter(|o| set.contains(&o.id)) {
        v.pos = v.pos + d;
        moved += 1;
    }
    for p in pcb.pours.iter_mut().filter(|o| set.contains(&o.id)) {
        p.outline.iter_mut().for_each(|q| *q = *q + d);
        for r in &mut p.filled {
            r.outer.iter_mut().for_each(|q| *q = *q + d);
            r.holes.iter_mut().flatten().for_each(|q| *q = *q + d);
        }
        moved += 1;
    }
    for g in pcb.graphics.iter_mut().filter(|o| set.contains(&o.id)) {
        g.shape = g.shape.transformed(&Transform::translate(d));
        moved += 1;
    }
    moved
}

pub fn pcb_rotate(pcb: &mut Pcb, ids: &[Id], c: Point, deg: f64) {
    let set: HashSet<Id> = ids.iter().copied().collect();
    for comp in pcb
        .components
        .iter_mut()
        .filter(|o| set.contains(&o.id) && !o.locked)
    {
        comp.pos = rot_about(comp.pos, c, deg);
        let delta = if comp.side == Side::Bottom { -deg } else { deg };
        comp.rotation = normalize_angle(comp.rotation + delta);
    }
    for t in pcb.tracks.iter_mut().filter(|o| set.contains(&o.id)) {
        t.a = rot_about(t.a, c, deg);
        t.b = rot_about(t.b, c, deg);
    }
    for v in pcb.vias.iter_mut().filter(|o| set.contains(&o.id)) {
        v.pos = rot_about(v.pos, c, deg);
    }
    for p in pcb.pours.iter_mut().filter(|o| set.contains(&o.id)) {
        p.outline
            .iter_mut()
            .for_each(|q| *q = rot_about(*q, c, deg));
        p.filled.clear();
    }
    let tf = Transform::new(c, deg, false);
    for g in pcb.graphics.iter_mut().filter(|o| set.contains(&o.id)) {
        g.shape = g
            .shape
            .transformed(&Transform::translate(-c))
            .transformed(&tf);
    }
}

/// Move footprints to the other side of the board, mirrored in place.
pub fn pcb_flip(pcb: &mut Pcb, ids: &[Id]) {
    for c in pcb
        .components
        .iter_mut()
        .filter(|o| ids.contains(&o.id) && !o.locked)
    {
        c.side = c.side.flip();
        c.rotation = normalize_angle(-c.rotation);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PcbFragment {
    pub components: Vec<PcbComponent>,
    pub tracks: Vec<Track>,
    pub vias: Vec<Via>,
    pub pours: Vec<Pour>,
    pub graphics: Vec<PcbGraphic>,
}

pub fn pcb_copy(pcb: &Pcb, ids: &[Id]) -> PcbFragment {
    let set: HashSet<Id> = ids.iter().copied().collect();
    PcbFragment {
        components: pcb
            .components
            .iter()
            .filter(|o| set.contains(&o.id))
            .cloned()
            .collect(),
        tracks: pcb
            .tracks
            .iter()
            .filter(|o| set.contains(&o.id))
            .cloned()
            .collect(),
        vias: pcb
            .vias
            .iter()
            .filter(|o| set.contains(&o.id))
            .cloned()
            .collect(),
        pours: pcb
            .pours
            .iter()
            .filter(|o| set.contains(&o.id))
            .cloned()
            .collect(),
        graphics: pcb
            .graphics
            .iter()
            .filter(|o| set.contains(&o.id))
            .cloned()
            .collect(),
    }
}

/// Paste board objects. Pasted footprints are not linked to the schematic.
pub fn pcb_paste(pcb: &mut Pcb, frag: &PcbFragment, d: Point, ids: &mut IdGen) -> Vec<Id> {
    let mut out = Vec::new();
    for c in &frag.components {
        let id = ids.fresh();
        out.push(id);
        pcb.components.push(PcbComponent {
            id,
            link: None,
            pos: c.pos + d,
            ..c.clone()
        });
    }
    for t in &frag.tracks {
        let id = ids.fresh();
        out.push(id);
        pcb.tracks.push(Track {
            id,
            a: t.a + d,
            b: t.b + d,
            ..t.clone()
        });
    }
    for v in &frag.vias {
        let id = ids.fresh();
        out.push(id);
        pcb.vias.push(Via {
            id,
            pos: v.pos + d,
            ..v.clone()
        });
    }
    for p in &frag.pours {
        let id = ids.fresh();
        out.push(id);
        pcb.pours.push(Pour {
            id,
            outline: p.outline.iter().map(|q| *q + d).collect(),
            filled: vec![],
            ..p.clone()
        });
    }
    for g in &frag.graphics {
        let id = ids.fresh();
        out.push(id);
        pcb.graphics.push(PcbGraphic {
            id,
            layer: g.layer,
            shape: g.shape.transformed(&Transform::translate(d)),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::examples::blinker;
    use crate::units::mil;

    #[test]
    fn moving_a_part_drags_its_wires() {
        let p = blinker();
        let lib = p.library.clone();
        let mut sheet = p.boards[0].schematic.sheets[0].clone();
        let r1 = sheet
            .components
            .iter()
            .find(|c| c.designator == "R1")
            .unwrap()
            .id;
        let before = crate::netlist::connectivity(
            &crate::schematic::Schematic {
                sheets: vec![sheet.clone()],
            },
            &lib,
        );
        sch_move(&mut sheet, &lib, &[r1], Point::new(mil(300.0), mil(200.0)));
        let after = crate::netlist::connectivity(
            &crate::schematic::Schematic {
                sheets: vec![sheet.clone()],
            },
            &lib,
        );
        assert_eq!(
            before.nets.len(),
            after.nets.len(),
            "connectivity preserved"
        );
        for w in &sheet.wires {
            for s in w.points.windows(2) {
                assert!(
                    s[0].x == s[1].x || s[0].y == s[1].y,
                    "wires stay orthogonal: {:?}",
                    w.points
                );
            }
        }
    }

    #[test]
    fn rotate_four_times_is_identity() {
        let p = blinker();
        let mut sheet = p.boards[0].schematic.sheets[0].clone();
        let ids: Vec<Id> = sheet
            .components
            .iter()
            .map(|c| c.id)
            .chain(sheet.wires.iter().map(|w| w.id))
            .collect();
        let orig = sheet.clone();
        for _ in 0..4 {
            sch_rotate(&mut sheet, &ids, Point::new(mil(5000.0), mil(5000.0)), 90.0);
        }
        assert_eq!(sheet.components, orig.components);
        assert_eq!(sheet.wires, orig.wires);
        sch_mirror(&mut sheet, &ids, Point::ZERO);
        sch_mirror(&mut sheet, &ids, Point::ZERO);
        assert_eq!(sheet.components, orig.components);
    }

    #[test]
    fn copy_paste_assigns_new_ids() {
        let mut p = blinker();
        let sheet = &mut p.boards[0].schematic.sheets[0];
        let ids: Vec<Id> = sheet.components.iter().take(2).map(|c| c.id).collect();
        let frag = sch_copy(sheet, &ids);
        let n = sheet.components.len();
        let new = sch_paste(sheet, &frag, Point::new(mil(1000.0), 0), &mut p.ids);
        assert_eq!(new.len(), 2);
        assert_eq!(sheet.components.len(), n + 2);
        assert!(sheet.components[n].designator.ends_with('?'));
        let json = serde_json::to_string(&frag).unwrap();
        assert_eq!(serde_json::from_str::<SchFragment>(&json).unwrap(), frag);
    }

    #[test]
    fn pcb_ops() {
        let mut p = blinker();
        let pcb = &mut p.boards[0].pcb;
        let id = pcb.components[0].id;
        let pos = pcb.components[0].pos;
        let rot0 = pcb.components[0].rotation;
        pcb_move(pcb, &[id], Point::new(1000, 0));
        assert_eq!(pcb.components[0].pos, pos + Point::new(1000, 0));
        pcb.components[0].locked = true;
        assert_eq!(pcb_move(pcb, &[id], Point::new(1000, 0)), 0);
        pcb.components[0].locked = false;
        pcb_flip(pcb, &[id]);
        assert_eq!(pcb.components[0].side, Side::Bottom);
        pcb_rotate(pcb, &[id], pcb.components[0].pos, 90.0);
        // Bottom-side parts rotate the other way when viewed from the top.
        assert_eq!(pcb.components[0].rotation, normalize_angle(-rot0 - 90.0));
        let frag = pcb_copy(pcb, &[id]);
        let new = pcb_paste(pcb, &frag, Point::new(0, 1000), &mut p.ids);
        assert_eq!(new.len(), 1);
    }
}
