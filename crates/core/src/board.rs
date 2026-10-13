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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::mm;

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
