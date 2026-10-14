//! Assembly drawings: board outline, part bodies and designators for one
//! side, for hand assembly or as a manufacturing note.

use crate::svg::scene_to_svg;
use easypro_core::geom::{Point, Rect};
use easypro_core::layer::{Layer, Side};
use easypro_core::library::Library;
use easypro_core::pcb::Pcb;
use easypro_core::scene::{rgba, shape_items, shape_items_bottom, Item, Prim, Scene, SceneLayer};
use easypro_core::shape::{Align, Shape};
use easypro_core::units::mm;

pub fn assembly_scene(pcb: &Pcb, lib: &Library, side: Side) -> Scene {
    let ink = rgba(0x202020, 255);
    let soft = rgba(0x808080, 255);
    let mut v: Vec<Item> = Vec::new();
    if let Some(o) = pcb.outline() {
        v.push(Item {
            prim: Prim::Path {
                pts: o,
                w: mm(0.2),
                closed: true,
            },
            color: ink,
            owner: None,
        });
    }
    let fab = if side == Side::Top {
        Layer::TopAssembly
    } else {
        Layer::BottomAssembly
    };
    for c in pcb.components.iter().filter(|c| c.side == side) {
        let Some(fp) = lib.footprints.get(&c.footprint) else {
            continue;
        };
        let tf = c.transform();
        let mut body = Rect::EMPTY;
        let mut drew = false;
        for g in fp.graphics.iter().filter(|g| c.board_layer(g.layer) == fab) {
            let s = g.shape.transformed(&tf);
            body = body.union(s.bbox());
            shape_items(&s, soft, None, &mut v);
            drew = true;
        }
        if !drew {
            body = c.bbox(fp);
            v.push(Item {
                prim: Prim::Path {
                    pts: body.corners().to_vec(),
                    w: mm(0.1),
                    closed: true,
                },
                color: soft,
                owner: None,
            });
        }
        if body.is_empty() {
            continue;
        }
        // Designator sized to the body, centred.
        let size = (body.height().min(body.width()) / 2).clamp(mm(0.5), mm(2.0));
        let text = Shape::Text {
            pos: body.center() - Point::new(0, size / 2),
            text: c.designator.clone(),
            size,
            rotation: 0.0,
            align: Align::Center,
        };
        if side == Side::Bottom {
            shape_items_bottom(&text, ink, None, &mut v);
        } else {
            shape_items(&text, ink, None, &mut v);
        }
    }
    let mut s = Scene::default();
    s.layers.push((SceneLayer::Schematic, v));
    s
}

/// SVG assembly drawing on white.
pub fn assembly_svg(pcb: &Pcb, lib: &Library, side: Side) -> String {
    scene_to_svg(&assembly_scene(pcb, lib, side), Some(0xffffff), mm(3.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_drawing_has_every_top_part() {
        let p = easypro_core::examples::blinker();
        let s = assembly_scene(&p.boards[0].pcb, &p.library, Side::Top);
        assert!(s.item_count() > 30);
        let svg = assembly_svg(&p.boards[0].pcb, &p.library, Side::Top);
        assert!(svg.starts_with("<svg"));
        let bottom = assembly_scene(&p.boards[0].pcb, &p.library, Side::Bottom);
        assert_eq!(bottom.item_count(), 1, "only the outline: no bottom parts");
    }
}
