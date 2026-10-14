//! Footprint editor: pads and silkscreen/assembly graphics.

use super::canvas::{draw_items, Built, Handler, Kind, View};
use crate::state::{AppState, Tool};
use easypro_core::geom::{normalize_angle, Point, Rect, Transform};
use easypro_core::id::Id;
use easypro_core::layer::Layer;
use easypro_core::library::{Footprint, FpGraphic, Pad, PadKind, PadShape};
use easypro_core::project::Project;
use easypro_core::scene::{self, rgba, Item, Prim, SceneLayer};
use easypro_core::shape::Shape;
use easypro_core::units::mm;
use easypro_render::gl::{GridStyle, LayerDraw};
use easypro_render::tess::Mesh;
use leptos::prelude::*;

pub struct FpHandler;

pub const GFX: u64 = 100_000;
const SILK: i64 = 150_000;

fn fid(k: &Kind) -> &str {
    match k {
        Kind::Fp(id) => id,
        _ => "",
    }
}

pub fn edit_footprint(app: &AppState, id: &str, label: &str, f: impl FnOnce(&mut Footprint)) {
    let id = id.to_string();
    app.edit(label, |p| {
        if let Some(fp) = p.library.footprints.get_mut(&id) {
            f(fp);
        }
    });
}

fn split(ids: &[Id]) -> (Vec<usize>, Vec<usize>) {
    let mut pads: Vec<usize> = ids
        .iter()
        .filter(|i| i.0 < GFX)
        .map(|i| i.0 as usize)
        .collect();
    let mut gfx: Vec<usize> = ids
        .iter()
        .filter(|i| i.0 >= GFX)
        .map(|i| (i.0 - GFX) as usize)
        .collect();
    pads.sort_unstable();
    gfx.sort_unstable();
    (pads, gfx)
}

pub fn delete_items(app: &AppState, id: &str) {
    let (pads, gfx) = split(&app.selection.get_untracked());
    if pads.is_empty() && gfx.is_empty() {
        return;
    }
    edit_footprint(app, id, "Delete", |f| {
        for i in pads.iter().rev() {
            if *i < f.pads.len() {
                f.pads.remove(*i);
            }
        }
        for i in gfx.iter().rev() {
            if *i < f.graphics.len() {
                f.graphics.remove(*i);
            }
        }
    });
    app.selection.set(vec![]);
}

pub fn rotate_items(app: &AppState, id: &str) {
    let (pads, gfx) = split(&app.selection.get_untracked());
    edit_footprint(app, id, "Rotate", |f| {
        for i in pads {
            if let Some(p) = f.pads.get_mut(i) {
                p.rotation = normalize_angle(p.rotation + 90.0);
            }
        }
        let tf = Transform::new(Point::ZERO, 90.0, false);
        for i in gfx {
            if let Some(g) = f.graphics.get_mut(i) {
                g.shape = g.shape.transformed(&tf);
            }
        }
    });
}

fn next_pad_number(f: &Footprint) -> String {
    let max = f
        .pads
        .iter()
        .filter_map(|p| p.number.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    (max + 1).to_string()
}

impl FpHandler {
    fn fp(&self, app: &AppState, k: &Kind) -> Option<Footprint> {
        app.project
            .with_untracked(|p| p.library.footprints.get(fid(k)).cloned())
    }

    fn new_pad(&self, app: &AppState, k: &Kind, at: Point, tht: bool) -> Pad {
        let last = self.fp(app, k).and_then(|f| {
            f.pads
                .iter()
                .rev()
                .find(|p| matches!(p.kind, PadKind::Tht { .. }) == tht)
                .cloned()
        });
        let n = self
            .fp(app, k)
            .map(|f| next_pad_number(&f))
            .unwrap_or_else(|| "1".into());
        match last {
            Some(p) => Pad {
                number: n,
                pos: at,
                ..p
            },
            None if tht => Pad {
                number: n,
                pos: at,
                size: Point::new(mm(1.7), mm(1.7)),
                shape: PadShape::Round,
                kind: PadKind::Tht { drill: mm(1.0) },
                rotation: 0.0,
            },
            None => Pad {
                number: n,
                pos: at,
                size: Point::new(mm(1.0), mm(1.2)),
                shape: PadShape::RoundRect { ratio_pct: 25 },
                kind: PadKind::Smd,
                rotation: 0.0,
            },
        }
    }
}

impl Handler for FpHandler {
    fn build(&self, app: &AppState, k: &Kind) -> Option<Built> {
        let f = self.fp(app, k)?;
        let mut scene = scene::footprint_scene(&f);
        let mut picks = Vec::new();
        for (i, p) in f.pads.iter().enumerate() {
            picks.push(Item {
                prim: Prim::Poly {
                    outer: p.hull(&Transform::default()).to_polygon(),
                    holes: vec![],
                },
                color: rgba(0xffffff, 255),
                owner: Some(Id(i as u64)),
            });
        }
        for (j, g) in f.graphics.iter().enumerate() {
            scene::shape_items(
                &g.shape,
                rgba(0xffffff, 255),
                Some(Id(GFX + j as u64)),
                &mut picks,
            );
        }
        scene.layers.push((SceneLayer::Overlay, picks));
        // Pad numbers and origin marker.
        let mut marks = Vec::new();
        for p in &f.pads {
            let size = (p.size.x.min(p.size.y) / 2).clamp(mm(0.2), mm(1.0));
            scene::shape_items(
                &Shape::Text {
                    pos: p.pos - Point::new(0, size / 2),
                    text: p.number.clone(),
                    size,
                    rotation: 0.0,
                    align: easypro_core::shape::Align::Center,
                },
                rgba(0xffffff, 230),
                None,
                &mut marks,
            );
        }
        let o = mm(0.6);
        marks.push(Item {
            prim: Prim::Seg {
                a: Point::new(-o, 0),
                b: Point::new(o, 0),
                w: 0,
            },
            color: rgba(0xa0a0a0, 255),
            owner: None,
        });
        marks.push(Item {
            prim: Prim::Seg {
                a: Point::new(0, -o),
                b: Point::new(0, o),
                w: 0,
            },
            color: rgba(0xa0a0a0, 255),
            owner: None,
        });
        scene.layers.push((SceneLayer::Frame, marks));
        let order = scene
            .layers
            .iter()
            .filter(|(l, _)| !matches!(l, SceneLayer::Overlay))
            .map(|(l, _)| LayerDraw {
                key: *l,
                alpha: if matches!(l, SceneLayer::Pcb(x) if x.is_copper()) {
                    0.85
                } else {
                    1.0
                },
            })
            .collect();
        let b = f.bbox();
        Some(Built {
            fit: if b.is_empty() {
                Rect::around(Point::ZERO, mm(5.0), mm(5.0))
            } else {
                b.inflate(mm(2.0))
            },
            scene,
            background: [0.07, 0.075, 0.085],
            grid: Some(GridStyle {
                step_mm: app.settings.with_untracked(|s| s.pcb_grid) as f64 / 1e6,
                lines: app.settings.with_untracked(|s| s.grid_lines),
                color: [0.2, 0.21, 0.24, 1.0],
                major: [0.28, 0.29, 0.33, 1.0],
            }),
            order,
        })
    }

    fn snap(&self, app: &AppState) -> i64 {
        app.settings.with_untracked(|s| s.pcb_grid) / 5
    }

    fn hit(&self, app: &AppState, k: &Kind, p: Point, tol: i64) -> Option<Id> {
        let f = self.fp(app, k)?;
        let probe = easypro_core::library::Hull::circle(p, 0);
        for (i, pad) in f.pads.iter().enumerate().rev() {
            if pad.hull(&Transform::default()).distance(&probe) <= tol as f64 {
                return Some(Id(i as u64));
            }
        }
        f.graphics
            .iter()
            .enumerate()
            .rev()
            .find(|(_, g)| g.shape.distance_to(p) <= tol as f64)
            .map(|(j, _)| Id(GFX + j as u64))
    }

    fn in_rect(&self, app: &AppState, k: &Kind, r: Rect, crossing: bool) -> Vec<Id> {
        let Some(f) = self.fp(app, k) else {
            return vec![];
        };
        let test = |b: Rect| {
            if crossing {
                r.intersects(&b)
            } else {
                r.contains_rect(&b)
            }
        };
        let mut out: Vec<Id> = f
            .pads
            .iter()
            .enumerate()
            .filter(|(_, p)| test(p.hull(&Transform::default()).bbox()))
            .map(|(i, _)| Id(i as u64))
            .collect();
        out.extend(
            f.graphics
                .iter()
                .enumerate()
                .filter(|(_, g)| test(g.shape.bbox()))
                .map(|(j, _)| Id(GFX + j as u64)),
        );
        out
    }

    fn apply_move(&self, p: &mut Project, k: &Kind, ids: &[Id], d: Point) {
        let (pads, gfx) = split(ids);
        if let Some(f) = p.library.footprints.get_mut(fid(k)) {
            for i in pads {
                if let Some(pad) = f.pads.get_mut(i) {
                    pad.pos = pad.pos + d;
                }
            }
            for j in gfx {
                if let Some(g) = f.graphics.get_mut(j) {
                    g.shape = g.shape.transformed(&Transform::translate(d));
                }
            }
        }
    }

    fn tool_down(
        &self,
        app: &AppState,
        k: &Kind,
        v: &mut View,
        _ev: &web_sys::PointerEvent,
    ) -> bool {
        let at = v.cursor;
        match app.tool.get_untracked() {
            Tool::PadSmd | Tool::PadTht => {
                let pad = self.new_pad(app, k, at, app.tool.get_untracked() == Tool::PadTht);
                let n = pad.number.clone();
                edit_footprint(app, fid(k), "Add pad", |f| f.pads.push(pad));
                app.status.set(format!("Pad {n} added"));
                true
            }
            Tool::FpLine => {
                v.tool.points.push(at);
                true
            }
            Tool::FpRect | Tool::FpCircle => {
                v.tool.points = vec![at];
                true
            }
            _ => false,
        }
    }

    fn tool_up(&self, app: &AppState, k: &Kind, v: &mut View) {
        let tool = app.tool.get_untracked();
        if !matches!(tool, Tool::FpRect | Tool::FpCircle) || v.tool.points.len() != 1 {
            return;
        }
        let a = v.tool.points[0];
        let b = v.cursor;
        v.tool = Default::default();
        if a == b {
            return;
        }
        let shape = if tool == Tool::FpRect {
            Shape::Rect {
                a,
                b,
                width: SILK,
                fill: false,
            }
        } else {
            Shape::Circle {
                center: a,
                radius: a.dist(b) as i64,
                width: SILK,
                fill: false,
            }
        };
        edit_footprint(app, fid(k), "Draw shape", |f| {
            f.graphics.push(FpGraphic {
                layer: Layer::TopSilk,
                shape,
            })
        });
    }

    fn tool_dbl(&self, app: &AppState, k: &Kind, v: &mut View) {
        if app.tool.get_untracked() == Tool::FpLine {
            let mut pts = std::mem::take(&mut v.tool.points);
            pts.dedup();
            if pts.len() >= 2 {
                edit_footprint(app, fid(k), "Draw line", |f| {
                    f.graphics.push(FpGraphic {
                        layer: Layer::TopSilk,
                        shape: Shape::Polyline {
                            points: pts,
                            width: SILK,
                        },
                    })
                });
            }
        }
    }

    fn overlay(&self, app: &AppState, k: &Kind, v: &View, m: &mut Mesh) {
        let mut items = Vec::new();
        let silk = rgba(Layer::TopSilk.color(), 230);
        match app.tool.get_untracked() {
            Tool::PadSmd | Tool::PadTht => {
                let pad = self.new_pad(app, k, v.cursor, app.tool.get_untracked() == Tool::PadTht);
                items.push(Item {
                    prim: Prim::Poly {
                        outer: pad.hull(&Transform::default()).to_polygon(),
                        holes: vec![],
                    },
                    color: rgba(Layer::TopCopper.color(), 140),
                    owner: None,
                });
            }
            Tool::FpLine if !v.tool.points.is_empty() => {
                let mut pts = v.tool.points.clone();
                pts.push(v.cursor);
                items.push(Item {
                    prim: Prim::Path {
                        pts,
                        w: SILK,
                        closed: false,
                    },
                    color: silk,
                    owner: None,
                });
            }
            Tool::FpRect if v.tool.points.len() == 1 => {
                items.push(Item {
                    prim: Prim::Path {
                        pts: Rect::new(v.tool.points[0], v.cursor).corners().to_vec(),
                        w: SILK,
                        closed: true,
                    },
                    color: silk,
                    owner: None,
                });
            }
            Tool::FpCircle if v.tool.points.len() == 1 => {
                let c = v.tool.points[0];
                items.push(Item {
                    prim: Prim::Ring {
                        c,
                        r: c.dist(v.cursor) as i64,
                        w: SILK,
                    },
                    color: silk,
                    owner: None,
                });
            }
            _ => {}
        }
        draw_items(v, m, &items);
    }

    fn open_item(&self, app: &AppState, _k: &Kind, id: Id) {
        app.selection.set(vec![id]);
        app.show_right.set(true);
    }
}
