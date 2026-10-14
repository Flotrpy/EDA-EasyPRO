//! Symbol editor: pins and body graphics.

use super::canvas::{draw_items, Built, Handler, Kind, View};
use crate::state::{AppState, Tool};
use easypro_core::geom::{normalize_angle, Point, Rect, Transform};
use easypro_core::id::Id;
use easypro_core::library::{Pin, PinKind, Symbol};
use easypro_core::project::Project;
use easypro_core::scene::{self, rgba, Item, Prim, Scene, SceneLayer};
use easypro_core::shape::Shape;
use easypro_core::units::mil;
use easypro_render::gl::{GridStyle, LayerDraw};
use easypro_render::tess::Mesh;
use leptos::prelude::*;

pub struct SymHandler;

/// Graphics are numbered after pins in the selection id space.
pub const GFX: u64 = 100_000;
const W: i64 = 152_400;

fn sid(k: &Kind) -> &str {
    match k {
        Kind::Sym(id) => id,
        _ => "",
    }
}

pub fn edit_symbol(app: &AppState, id: &str, label: &str, f: impl FnOnce(&mut Symbol)) {
    let id = id.to_string();
    app.edit(label, |p| {
        if let Some(s) = p.library.symbols.get_mut(&id) {
            f(s);
        }
    });
}

fn split(ids: &[Id]) -> (Vec<usize>, Vec<usize>) {
    let mut pins: Vec<usize> = ids
        .iter()
        .filter(|i| i.0 < GFX)
        .map(|i| i.0 as usize)
        .collect();
    let mut gfx: Vec<usize> = ids
        .iter()
        .filter(|i| i.0 >= GFX)
        .map(|i| (i.0 - GFX) as usize)
        .collect();
    pins.sort_unstable();
    gfx.sort_unstable();
    (pins, gfx)
}

pub fn delete_items(app: &AppState, id: &str) {
    let (pins, gfx) = split(&app.selection.get_untracked());
    if pins.is_empty() && gfx.is_empty() {
        return;
    }
    edit_symbol(app, id, "Delete", |s| {
        for i in pins.iter().rev() {
            if *i < s.pins.len() {
                s.pins.remove(*i);
            }
        }
        for i in gfx.iter().rev() {
            if *i < s.graphics.len() {
                s.graphics.remove(*i);
            }
        }
    });
    app.selection.set(vec![]);
}

pub fn rotate_items(app: &AppState, id: &str) {
    let (pins, gfx) = split(&app.selection.get_untracked());
    edit_symbol(app, id, "Rotate", |s| {
        for i in pins {
            if let Some(p) = s.pins.get_mut(i) {
                // Rotate the pin about its connection point.
                p.angle = normalize_angle(p.angle + 90.0);
            }
        }
        let tf = Transform::new(Point::ZERO, 90.0, false);
        for i in gfx {
            if let Some(g) = s.graphics.get_mut(i) {
                *g = g.transformed(&tf);
            }
        }
    });
}

fn next_pin_number(s: &Symbol) -> String {
    let max = s
        .pins
        .iter()
        .filter_map(|p| p.number.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    (max + 1).to_string()
}

fn pin_items(pin: &Pin, owner: Id, color: u32, out: &mut Vec<Item>) {
    out.push(Item {
        prim: Prim::Seg {
            a: pin.pos,
            b: pin.body_end(),
            w: W * 2,
        },
        color,
        owner: Some(owner),
    });
    out.push(Item {
        prim: Prim::Ring {
            c: pin.pos,
            r: mil(15.0),
            w: W,
        },
        color,
        owner: Some(owner),
    });
}

impl SymHandler {
    fn symbol(&self, app: &AppState, k: &Kind) -> Option<Symbol> {
        app.project
            .with_untracked(|p| p.library.symbols.get(sid(k)).cloned())
    }
}

impl Handler for SymHandler {
    fn build(&self, app: &AppState, k: &Kind) -> Option<Built> {
        let sym = self.symbol(app, k)?;
        let dark = app.dark.get_untracked();
        let th = super::sch::theme(app);
        let mut scene = scene::symbol_scene(&sym, &th);
        // Connection points and the origin.
        let mut marks = Vec::new();
        for p in &sym.pins {
            marks.push(Item {
                prim: Prim::Ring {
                    c: p.pos,
                    r: mil(12.0),
                    w: 0,
                },
                color: rgba(0x2060ff, 255),
                owner: None,
            });
        }
        let o = mil(40.0);
        marks.push(Item {
            prim: Prim::Seg {
                a: Point::new(-o, 0),
                b: Point::new(o, 0),
                w: 0,
            },
            color: rgba(0x909090, 255),
            owner: None,
        });
        marks.push(Item {
            prim: Prim::Seg {
                a: Point::new(0, -o),
                b: Point::new(0, o),
                w: 0,
            },
            color: rgba(0x909090, 255),
            owner: None,
        });
        scene.layers.push((SceneLayer::Frame, marks));
        // Per-item geometry used only for picking highlights.
        let mut picks = Vec::new();
        for (i, p) in sym.pins.iter().enumerate() {
            pin_items(p, Id(i as u64), th.pin, &mut picks);
        }
        for (j, g) in sym.graphics.iter().enumerate() {
            scene::shape_items(g, th.body, Some(Id(GFX + j as u64)), &mut picks);
        }
        scene.layers.push((SceneLayer::Overlay, picks));
        let fit = sym.bbox().inflate(mil(300.0));
        Some(Built {
            fit: if sym.pins.is_empty() && sym.graphics.is_empty() {
                Rect::around(Point::ZERO, mil(500.0), mil(500.0))
            } else {
                fit
            },
            scene: Scene {
                layers: scene.layers,
            },
            background: if dark {
                [0.118, 0.122, 0.133]
            } else {
                [1.0, 1.0, 1.0]
            },
            grid: Some(GridStyle {
                step_mm: app.settings.with_untracked(|s| s.sch_grid) as f64 / 1e6,
                lines: app.settings.with_untracked(|s| s.grid_lines),
                color: if dark {
                    [0.3, 0.31, 0.34, 1.0]
                } else {
                    [0.80, 0.82, 0.86, 1.0]
                },
                major: if dark {
                    [0.36, 0.37, 0.41, 1.0]
                } else {
                    [0.70, 0.72, 0.77, 1.0]
                },
            }),
            order: vec![
                LayerDraw {
                    key: SceneLayer::Schematic,
                    alpha: 1.0,
                },
                LayerDraw {
                    key: SceneLayer::Frame,
                    alpha: 1.0,
                },
            ],
        })
    }

    fn snap(&self, app: &AppState) -> i64 {
        app.settings.with_untracked(|s| s.sch_grid)
    }

    fn hit(&self, app: &AppState, k: &Kind, p: Point, tol: i64) -> Option<Id> {
        let s = self.symbol(app, k)?;
        for (i, pin) in s.pins.iter().enumerate() {
            if easypro_core::geom::point_segment_dist(p, pin.pos, pin.body_end()) <= tol as f64 {
                return Some(Id(i as u64));
            }
        }
        s.graphics
            .iter()
            .enumerate()
            .rev()
            .find(|(_, g)| g.distance_to(p) <= tol as f64)
            .map(|(j, _)| Id(GFX + j as u64))
    }

    fn in_rect(&self, app: &AppState, k: &Kind, r: Rect, crossing: bool) -> Vec<Id> {
        let Some(s) = self.symbol(app, k) else {
            return vec![];
        };
        let test = |b: Rect| {
            if crossing {
                r.intersects(&b)
            } else {
                r.contains_rect(&b)
            }
        };
        let mut out: Vec<Id> = s
            .pins
            .iter()
            .enumerate()
            .filter(|(_, p)| test(Rect::new(p.pos, p.body_end())))
            .map(|(i, _)| Id(i as u64))
            .collect();
        out.extend(
            s.graphics
                .iter()
                .enumerate()
                .filter(|(_, g)| test(g.bbox()))
                .map(|(j, _)| Id(GFX + j as u64)),
        );
        out
    }

    fn apply_move(&self, p: &mut Project, k: &Kind, ids: &[Id], d: Point) {
        let (pins, gfx) = split(ids);
        if let Some(s) = p.library.symbols.get_mut(sid(k)) {
            for i in pins {
                if let Some(pin) = s.pins.get_mut(i) {
                    pin.pos = pin.pos + d;
                }
            }
            for j in gfx {
                if let Some(g) = s.graphics.get_mut(j) {
                    *g = g.transformed(&Transform::translate(d));
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
            Tool::Pin => {
                let angle = app.place_rot.get_untracked();
                edit_symbol(app, sid(k), "Add pin", |s| {
                    let n = next_pin_number(s);
                    s.pins.push(Pin {
                        number: n.clone(),
                        name: n,
                        pos: at,
                        angle,
                        length: mil(100.0),
                        kind: PinKind::Passive,
                        hide_name: false,
                    });
                });
                true
            }
            Tool::SymLine => {
                v.tool.points.push(at);
                true
            }
            Tool::SymRect | Tool::SymCircle => {
                v.tool.points = vec![at];
                true
            }
            _ => false,
        }
    }

    fn tool_up(&self, app: &AppState, k: &Kind, v: &mut View) {
        let tool = app.tool.get_untracked();
        if !matches!(tool, Tool::SymRect | Tool::SymCircle) || v.tool.points.len() != 1 {
            return;
        }
        let a = v.tool.points[0];
        let b = v.cursor;
        v.tool = Default::default();
        if a == b {
            return;
        }
        let shape = if tool == Tool::SymRect {
            Shape::Rect {
                a,
                b,
                width: W,
                fill: false,
            }
        } else {
            Shape::Circle {
                center: a,
                radius: a.dist(b) as i64,
                width: W,
                fill: false,
            }
        };
        edit_symbol(app, sid(k), "Draw shape", |s| s.graphics.push(shape));
    }

    fn tool_dbl(&self, app: &AppState, k: &Kind, v: &mut View) {
        if app.tool.get_untracked() == Tool::SymLine {
            let mut pts = std::mem::take(&mut v.tool.points);
            pts.dedup();
            if pts.len() >= 2 {
                edit_symbol(app, sid(k), "Draw line", |s| {
                    s.graphics.push(Shape::Polyline {
                        points: pts,
                        width: W,
                    })
                });
            }
        }
    }

    fn overlay(&self, app: &AppState, _k: &Kind, v: &View, m: &mut Mesh) {
        let th = super::sch::theme(app);
        let mut items = Vec::new();
        match app.tool.get_untracked() {
            Tool::Pin => {
                let angle = app.place_rot.get_untracked();
                let pin = Pin {
                    number: String::new(),
                    name: String::new(),
                    pos: v.cursor,
                    angle,
                    length: mil(100.0),
                    kind: PinKind::Passive,
                    hide_name: true,
                };
                pin_items(&pin, Id(0), th.pin, &mut items);
            }
            Tool::SymLine if !v.tool.points.is_empty() => {
                let mut pts = v.tool.points.clone();
                pts.push(v.cursor);
                items.push(Item {
                    prim: Prim::Path {
                        pts,
                        w: W,
                        closed: false,
                    },
                    color: th.body,
                    owner: None,
                });
            }
            Tool::SymRect if v.tool.points.len() == 1 => {
                items.push(Item {
                    prim: Prim::Path {
                        pts: Rect::new(v.tool.points[0], v.cursor).corners().to_vec(),
                        w: W,
                        closed: true,
                    },
                    color: th.body,
                    owner: None,
                });
            }
            Tool::SymCircle if v.tool.points.len() == 1 => {
                let c = v.tool.points[0];
                items.push(Item {
                    prim: Prim::Ring {
                        c,
                        r: c.dist(v.cursor) as i64,
                        w: W,
                    },
                    color: th.body,
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
