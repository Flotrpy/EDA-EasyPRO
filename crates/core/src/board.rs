//! Board-level helpers used by Board Setup: outlines and mounting holes.

use crate::geom::{arc_points, Point, Rect};
use crate::id::IdGen;
use crate::layer::{Layer, Side};
use crate::library::Library;
use crate::pcb::{Pcb, PcbComponent, PcbGraphic};
use crate::shape::Shape;
use std::collections::BTreeMap;

/// A rectangle with rounded corners as a polygon.
pub fn rounded_rect(r: Rect, radius: i64) -> Vec<Point> {
    let rad = radius.clamp(0, r.width().min(r.height()) / 2);
    if rad == 0 {
        return r.corners().to_vec();
    }
    let mut pts = Vec::new();
    let corners = [
        (Point::new(r.max.x - rad, r.min.y + rad), 270.0),
        (Point::new(r.max.x - rad, r.max.y - rad), 0.0),
        (Point::new(r.min.x + rad, r.max.y - rad), 90.0),
        (Point::new(r.min.x + rad, r.min.y + rad), 180.0),
    ];
    for (c, start) in corners {
        pts.extend(arc_points(c, rad, start, 90.0, 10.0));
    }
    pts.dedup();
    pts
}

/// Replace the board outline with a (rounded) rectangle.
pub fn set_rect_outline(pcb: &mut Pcb, r: Rect, radius: i64, ids: &mut IdGen) {
    pcb.graphics.retain(|g| g.layer != Layer::Outline);
    pcb.graphics.push(PcbGraphic {
        id: ids.fresh(),
        layer: Layer::Outline,
        shape: Shape::Polygon {
            points: rounded_rect(r, radius),
            width: 100_000,
            fill: false,
        },
    });
}

/// Put a mounting hole `inset` from each corner of the outline's bounding
/// box. The device and footprint must already be in `lib`. Returns how many
/// holes were added (corners that already have a hole are skipped).
pub fn add_corner_holes(
    pcb: &mut Pcb,
    lib: &Library,
    device: &str,
    inset: i64,
    ids: &mut IdGen,
) -> usize {
    let Some(outline) = pcb.outline() else {
        return 0;
    };
    let Some(dev) = lib.devices.get(device) else {
        return 0;
    };
    let Some(fp) = dev.footprint.clone() else {
        return 0;
    };
    let b = Rect::from_points(outline);
    let spots = [
        Point::new(b.min.x + inset, b.min.y + inset),
        Point::new(b.max.x - inset, b.min.y + inset),
        Point::new(b.max.x - inset, b.max.y - inset),
        Point::new(b.min.x + inset, b.max.y - inset),
    ];
    let mut n = pcb
        .components
        .iter()
        .filter(|c| c.designator.starts_with('H'))
        .count();
    let mut added = 0;
    for at in spots {
        if pcb
            .components
            .iter()
            .any(|c| c.footprint == fp && c.pos.dist(at) < inset as f64 / 2.0)
        {
            continue;
        }
        n += 1;
        pcb.components.push(PcbComponent {
            id: ids.fresh(),
            link: None,
            designator: format!("H{n}"),
            value: dev.value.clone(),
            device: device.to_string(),
            footprint: fp.clone(),
            pos: at,
            rotation: 0.0,
            side: Side::Top,
            locked: true,
            pad_nets: BTreeMap::new(),
            designator_offset: None,
            show_designator: false,
        });
        added += 1;
    }
    added
}

/// Summary numbers for the Board Info dialog.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoardStats {
    pub width: i64,
    pub height: i64,
    /// Area inside the outline in mm².
    pub area_mm2: f64,
    pub components_top: usize,
    pub components_bottom: usize,
    pub smd_pads: usize,
    pub tht_pads: usize,
    pub vias: usize,
    pub tracks: usize,
    /// Total routed track length in mm.
    pub track_length_mm: f64,
    /// Every drilled hole: plated and non-plated pads plus vias.
    pub holes: usize,
    pub smallest_drill: Option<i64>,
    pub nets: usize,
    /// Routed length per net in mm, longest first.
    pub net_lengths: Vec<(String, f64)>,
}

pub fn stats(pcb: &Pcb, lib: &Library) -> BoardStats {
    use crate::library::PadKind;
    let mut s = BoardStats::default();
    if let Some(o) = pcb.outline() {
        let r = Rect::from_points(o.iter().copied());
        s.width = r.width();
        s.height = r.height();
        s.area_mm2 = crate::geom::polygon_area(&o).abs() / 1e12;
    }
    let drill = |d: i64, s: &mut BoardStats| {
        s.holes += 1;
        s.smallest_drill = Some(s.smallest_drill.map_or(d, |m: i64| m.min(d)));
    };
    for (_, pad, _) in pcb.pads(lib) {
        match pad.kind {
            PadKind::Smd => s.smd_pads += 1,
            PadKind::Tht { drill: d } => {
                s.tht_pads += 1;
                drill(d, &mut s);
            }
            PadKind::Npth { drill: d } => drill(d, &mut s),
        }
    }
    for c in &pcb.components {
        match c.side {
            Side::Top => s.components_top += 1,
            Side::Bottom => s.components_bottom += 1,
        }
    }
    for v in &pcb.vias {
        drill(v.drill, &mut s);
    }
    s.vias = pcb.vias.len();
    s.tracks = pcb.tracks.len();
    let mut per_net: BTreeMap<String, f64> = BTreeMap::new();
    for t in &pcb.tracks {
        let l = t.length() / 1e6;
        s.track_length_mm += l;
        if !t.net.is_empty() {
            *per_net.entry(t.net.clone()).or_default() += l;
        }
    }
    s.nets = pcb.nets().len();
    s.net_lengths = per_net.into_iter().collect();
    s.net_lengths
        .sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::mm;

    #[test]
    fn regulator_stats() {
        let p = crate::examples::regulator();
        let s = stats(&p.boards[0].pcb, &p.library);
        assert_eq!((s.width, s.height), (mm(45.0), mm(22.0)));
        // Rounded corners shave a little off 990 mm².
        assert!(s.area_mm2 > 985.0 && s.area_mm2 < 990.0, "{}", s.area_mm2);
        assert_eq!((s.components_top, s.components_bottom), (7, 0));
        assert_eq!(s.tht_pads, 4);
        assert_eq!(s.vias, 4);
        assert_eq!(s.holes, 8);
        assert_eq!(s.smallest_drill, Some(mm(0.3)));
        assert_eq!(s.nets, 4);
        assert_eq!(s.net_lengths[0].0, "+3V3");
        let sum: f64 = s.net_lengths.iter().map(|n| n.1).sum();
        assert!((sum - s.track_length_mm).abs() < 1e-9);
    }

    #[test]
    fn rounded_outline_stays_inside() {
        let r = Rect::new(Point::ZERO, Point::new(mm(50.0), mm(30.0)));
        let pts = rounded_rect(r, mm(3.0));
        assert!(pts.len() > 8);
        assert!(pts.iter().all(|p| r.contains(*p)));
        assert!(!pts.contains(&Point::ZERO), "corners are cut");
        assert_eq!(rounded_rect(r, 0).len(), 4);
    }

    #[test]
    fn corner_holes_are_added_once() {
        let mut p = crate::project::Project::new("t");
        p.library
            .import_device(crate::builtin::library(), "dev:MH-M3");
        let lib = p.library.clone();
        let mut ids = p.ids.clone();
        let pcb = &mut p.boards[0].pcb;
        assert_eq!(
            add_corner_holes(pcb, &lib, "dev:MH-M3", mm(3.5), &mut ids),
            4
        );
        assert_eq!(
            add_corner_holes(pcb, &lib, "dev:MH-M3", mm(3.5), &mut ids),
            0
        );
        let drc = crate::drc::run(pcb, &lib, crate::units::Unit::Mm);
        assert!(
            !drc.iter()
                .any(|i| i.severity == crate::check::Severity::Error),
            "{drc:#?}"
        );
    }

    #[test]
    fn outline_replacement() {
        let mut p = crate::project::Project::new("t");
        let mut ids = p.ids.clone();
        let pcb = &mut p.boards[0].pcb;
        set_rect_outline(
            pcb,
            Rect::new(Point::ZERO, Point::new(mm(80.0), mm(60.0))),
            mm(2.0),
            &mut ids,
        );
        assert_eq!(
            pcb.graphics
                .iter()
                .filter(|g| g.layer == Layer::Outline)
                .count(),
            1
        );
        let o = Rect::from_points(pcb.outline().unwrap());
        assert_eq!(o.width(), mm(80.0));
    }
}
