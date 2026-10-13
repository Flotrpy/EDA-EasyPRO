//! PCB editor behaviour.

use super::canvas::{draw_items, Built, Handler, Kind, View};
use crate::state::{AppState, Tool};
use easypro_core::geom::{Point, Rect};
use easypro_core::id::Id;
use easypro_core::layer::Layer;
use easypro_core::pcb::{PcbGraphic, Pour, Track, Via};
use easypro_core::pick;
use easypro_core::project::Project;
use easypro_core::route::{best_path45, path45};
use easypro_core::scene::{self, rgba, Item, PcbStyle, Prim, SceneLayer};
use easypro_core::shape::{Align, Shape};
use easypro_render::gl::{GridStyle, LayerDraw};
use easypro_render::tess::Mesh;
use leptos::prelude::*;

pub struct PcbHandler;

fn board(k: &Kind) -> Id {
    match k {
        Kind::Pcb { board } => *board,
        _ => Id(0),
    }
}

fn silk_for(l: Layer) -> Layer {
    if l == Layer::BottomCopper {
        Layer::BottomSilk
    } else {
        Layer::TopSilk
    }
}

impl PcbHandler {
    fn edit_pcb(
        &self,
        app: &AppState,
        k: &Kind,
        label: &str,
        rebuild: bool,
        f: impl FnOnce(&mut easypro_core::pcb::Pcb, &mut easypro_core::id::IdGen),
    ) {
        let b = board(k);
        app.edit(label, |p| {
            let lib = p.library.clone();
            let mut ids = std::mem::take(&mut p.ids);
            if let Some(bd) = p.board_mut(b) {
                f(&mut bd.pcb, &mut ids);
                if rebuild {
                    bd.pcb.rebuild_pours(&lib);
                }
            }
            p.ids = ids;
        });
    }

    fn rules(&self, app: &AppState, k: &Kind) -> easypro_core::pcb::DesignRules {
        app.project.with_untracked(|p| {
            p.board(board(k))
                .map(|b| b.pcb.rules.clone())
                .unwrap_or_default()
        })
    }

    fn anchor(&self, app: &AppState, k: &Kind, v: &View, layer: Layer) -> Option<(Point, String)> {
        let tol = v.tol() * 2;
        app.project.with_untracked(|p| {
            p.board(board(k))
                .and_then(|b| pick::pcb_anchor(&b.pcb, &p.library, v.raw, tol, layer))
        })
    }

    fn target(&self, app: &AppState, k: &Kind, v: &View) -> Point {
        let layer = app.active_layer.get_untracked();
        match self.anchor(app, k, v, layer) {
            Some((p, net)) if v.tool.net.is_empty() || net == v.tool.net || net.is_empty() => p,
            _ => v.cursor,
        }
    }

    /// Proposed route from the last point to the cursor and whether it is clear.
    fn proposal(&self, app: &AppState, k: &Kind, v: &View) -> (Vec<Point>, bool) {
        let Some(last) = v.tool.points.last().copied() else {
            return (vec![], true);
        };
        let target = self.target(app, k, v);
        let layer = app.active_layer.get_untracked();
        app.project.with_untracked(|p| {
            let Some(b) = p.board(board(k)) else {
                return (path45(last, target, v.tool.alt), true);
            };
            let items = b.pcb.copper_items(&p.library);
            let clearance = b.pcb.rules.clearance_for(&v.tool.net, "");
            best_path45(
                last,
                target,
                v.tool.alt,
                v.tool.width,
                layer,
                &v.tool.net,
                clearance,
                &items,
            )
        })
    }

    fn commit_route(&self, app: &AppState, k: &Kind, v: &mut View, force: bool) -> bool {
        let (path, ok) = self.proposal(app, k, v);
        if path.len() < 2 {
            return false;
        }
        if !ok && !force {
            app.warn("That route would violate clearance. Hold Ctrl to place it anyway, or try the other bend with /.");
            return false;
        }
        let layer = app.active_layer.get_untracked();
        let (net, width) = (v.tool.net.clone(), v.tool.width);
        let segs: Vec<(Point, Point)> = path
            .windows(2)
            .filter(|w| w[0] != w[1])
            .map(|w| (w[0], w[1]))
            .collect();
        self.edit_pcb(app, k, "Route track", false, |pcb, ids| {
            for (a, b) in segs {
                pcb.tracks.push(Track {
                    id: ids.fresh(),
                    layer,
                    a,
                    b,
                    width,
                    net: net.clone(),
                });
            }
        });
        let end = *path.last().unwrap();
        v.tool.points = vec![end];
        // Ending on a pad of the same net finishes the route.
        if let Some((p, n)) = self.anchor(app, k, v, layer) {
            if p == end && !n.is_empty() && n == v.tool.net && path.len() >= 2 {
                self.finish(app, k, v);
            }
        }
        true
    }

    fn finish(&self, app: &AppState, k: &Kind, v: &mut View) {
        let had = !v.tool.points.is_empty();
        v.tool = Default::default();
        if had {
            // Fills react to the new copper.
            let b = board(k);
            app.project.update(|p| {
                let lib = p.library.clone();
                if let Some(bd) = p.board_mut(b) {
                    if !bd.pcb.pours.is_empty() {
                        bd.pcb.rebuild_pours(&lib);
                    }
                }
            });
            app.rev.update(|r| *r += 1);
        }
    }
}

impl Handler for PcbHandler {
    fn build(&self, app: &AppState, k: &Kind) -> Option<Built> {
        let hidden = app.hidden_layers.get_untracked();
        let active = app.active_layer.get_untracked();
        let style = PcbStyle {
            hidden: hidden.clone(),
            highlight_net: app.highlight_net.get_untracked(),
            show_ratsnest: true,
        };
        app.project.with_untracked(|p| {
            let b = p.board(board(k))?;
            let conn = easypro_core::connect::analyze(&b.pcb, &p.library);
            let scene = scene::pcb_scene(&b.pcb, &p.library, &style, &conn.ratsnest);
            let mut order = Vec::new();
            let keys: Vec<SceneLayer> = scene.layers.iter().map(|(l, _)| *l).collect();
            for key in &keys {
                match key {
                    SceneLayer::Pcb(l) if *l == active => {}
                    SceneLayer::Pcb(l) if l.is_copper() && *l != Layer::Multi => {
                        order.push(LayerDraw {
                            key: *key,
                            alpha: 0.55,
                        })
                    }
                    SceneLayer::Pcb(Layer::Multi) | SceneLayer::Drill | SceneLayer::PadText => {}
                    SceneLayer::Pcb(_)
                    | SceneLayer::Ratsnest
                    | SceneLayer::Overlay
                    | SceneLayer::Frame
                    | SceneLayer::Schematic => {}
                }
            }
            order.push(LayerDraw {
                key: SceneLayer::Pcb(active),
                alpha: 1.0,
            });
            order.push(LayerDraw {
                key: SceneLayer::Pcb(Layer::Multi),
                alpha: 1.0,
            });
            order.push(LayerDraw {
                key: SceneLayer::Drill,
                alpha: 1.0,
            });
            order.push(LayerDraw {
                key: SceneLayer::PadText,
                alpha: 1.0,
            });
            for key in &keys {
                if let SceneLayer::Pcb(l) = key {
                    if !l.is_copper() {
                        order.push(LayerDraw {
                            key: *key,
                            alpha: 1.0,
                        });
                    }
                }
            }
            order.push(LayerDraw {
                key: SceneLayer::Ratsnest,
                alpha: 1.0,
            });
            let fit = b.pcb.bbox(&p.library);
            Some(Built {
                fit,
                scene,
                background: [0.07, 0.075, 0.085],
                grid: Some(GridStyle {
                    step_mm: app.settings.with_untracked(|s| s.pcb_grid) as f64 / 1e6,
                    lines: app.settings.with_untracked(|s| s.grid_lines),
                    color: [0.20, 0.21, 0.24, 1.0],
                    major: [0.28, 0.29, 0.33, 1.0],
                }),
                order,
            })
        })
    }

    fn snap(&self, app: &AppState) -> i64 {
        app.settings.with_untracked(|s| s.pcb_grid)
    }

    fn hit(&self, app: &AppState, k: &Kind, p: Point, tol: i64) -> Option<Id> {
        let hidden = app.hidden_layers.get_untracked();
        let active = app.active_layer.get_untracked();
        let vis = move |l: Layer| !hidden.contains(&l);
        app.project.with_untracked(|pr| {
            pr.board(board(k))
                .and_then(|b| pick::pcb_hit(&b.pcb, &pr.library, p, tol, active, &vis))
        })
    }

    fn in_rect(&self, app: &AppState, k: &Kind, r: Rect, crossing: bool) -> Vec<Id> {
        app.project.with_untracked(|pr| {
            pr.board(board(k))
                .map(|b| pick::pcb_in_rect(&b.pcb, &pr.library, r, crossing))
                .unwrap_or_default()
        })
    }

    fn apply_move(&self, p: &mut Project, k: &Kind, ids: &[Id], d: Point) {
        if let Some(b) = p.board_mut(board(k)) {
            easypro_core::edit::pcb_move(&mut b.pcb, ids, d);
        }
    }

    fn tool_down(
        &self,
        app: &AppState,
        k: &Kind,
        v: &mut View,
        ev: &web_sys::PointerEvent,
    ) -> bool {
        let layer = app.active_layer.get_untracked();
        match app.tool.get_untracked() {
            Tool::Track => {
                if v.tool.points.is_empty() {
                    let (start, net) = self
                        .anchor(app, k, v, layer)
                        .unwrap_or((v.cursor, String::new()));
                    let rules = self.rules(app, k);
                    v.tool.points = vec![start];
                    v.tool.width = app
                        .route_width
                        .get_untracked()
                        .unwrap_or_else(|| rules.track_width_for(&net));
                    v.tool.net = net.clone();
                    app.highlight_net
                        .set((!net.is_empty()).then_some(net.clone()));
                    app.status.set(if net.is_empty() {
                        "Routing a track with no net".into()
                    } else {
                        format!("Routing {net}")
                    });
                } else {
                    self.commit_route(app, k, v, ev.ctrl_key());
                }
                true
            }
            Tool::Via => {
                let rules = self.rules(app, k);
                let net = self
                    .anchor(app, k, v, Layer::TopCopper)
                    .or_else(|| self.anchor(app, k, v, Layer::BottomCopper))
                    .map(|(_, n)| n)
                    .unwrap_or_default();
                let (d, dr) = rules.via_for(&net);
                let at = v.cursor;
                self.edit_pcb(app, k, "Place via", true, |pcb, ids| {
                    pcb.vias.push(Via {
                        id: ids.fresh(),
                        pos: at,
                        diameter: d,
                        drill: dr,
                        net,
                    })
                });
                true
            }
            Tool::Pour | Tool::Line => {
                v.tool.points.push(v.cursor);
                true
            }
            Tool::Outline => {
                v.tool.points = vec![v.cursor];
                true
            }
            Tool::BoardText => {
                let text = web_sys::window().and_then(|w| {
                    w.prompt_with_message_and_default("Silkscreen text", "")
                        .ok()
                        .flatten()
                });
                let Some(text) = text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
                else {
                    return true;
                };
                let layer = silk_for(app.active_layer.get_untracked());
                let at = v.cursor;
                self.edit_pcb(app, k, "Add text", false, |pcb, ids| {
                    pcb.graphics.push(PcbGraphic {
                        id: ids.fresh(),
                        layer,
                        shape: Shape::Text {
                            pos: at,
                            text,
                            size: 1_000_000,
                            rotation: 0.0,
                            align: Align::Left,
                        },
                    })
                });
                true
            }
            Tool::Measure => {
                if v.tool.points.len() != 1 {
                    v.tool.points = vec![v.cursor];
                } else {
                    v.tool.points.push(v.cursor);
                    let d = v.tool.points[0].dist(v.tool.points[1]) as i64;
                    app.info(format!("Distance: {}", app.fmt_len(d)));
                }
                true
            }
            _ => false,
        }
    }

    fn tool_up(&self, app: &AppState, k: &Kind, v: &mut View) {
        if app.tool.get_untracked() == Tool::Outline && v.tool.points.len() == 1 {
            let a = v.tool.points[0];
            let b = v.cursor;
            v.tool = Default::default();
            if (a.x - b.x).abs() < 1_000_000 || (a.y - b.y).abs() < 1_000_000 {
                app.status
                    .set("Drag a larger rectangle to set the board outline".into());
                return;
            }
            let r = Rect::new(a, b);
            self.edit_pcb(app, k, "Set board outline", true, |pcb, ids| {
                pcb.graphics.retain(|g| g.layer != Layer::Outline);
                pcb.graphics.push(PcbGraphic {
                    id: ids.fresh(),
                    layer: Layer::Outline,
                    shape: Shape::Polygon {
                        points: r.corners().to_vec(),
                        width: 100_000,
                        fill: false,
                    },
                });
            });
            app.ok(format!(
                "Board outline set to {} x {}",
                app.fmt_len(r.width()),
                app.fmt_len(r.height())
            ));
        }
    }

    fn tool_dbl(&self, app: &AppState, k: &Kind, v: &mut View) {
        match app.tool.get_untracked() {
            Tool::Track => self.finish(app, k, v),
            Tool::Pour => {
                let mut pts = std::mem::take(&mut v.tool.points);
                pts.dedup();
                if pts.len() < 3 {
                    app.status
                        .set("A copper area needs at least three corners".into());
                    return;
                }
                let nets = app.project.with_untracked(|p| {
                    p.board(board(k)).map(|b| b.pcb.nets()).unwrap_or_default()
                });
                let default = if nets.iter().any(|n| n == "GND") {
                    "GND".to_string()
                } else {
                    nets.first().cloned().unwrap_or_default()
                };
                let net = web_sys::window().and_then(|w| {
                    w.prompt_with_message_and_default("Copper area net (empty for none)", &default)
                        .ok()
                        .flatten()
                });
                let Some(net) = net else { return };
                let layer = app.active_layer.get_untracked();
                self.edit_pcb(app, k, "Add copper area", true, |pcb, ids| {
                    pcb.pours.push(Pour {
                        id: ids.fresh(),
                        layer,
                        net: net.trim().to_string(),
                        outline: pts,
                        clearance: None,
                        remove_islands: true,
                        filled: vec![],
                    })
                });
                app.ok(format!("Copper area added on {}", layer.name()));
            }
            Tool::Line => {
                let mut pts = std::mem::take(&mut v.tool.points);
                pts.dedup();
                if pts.len() >= 2 {
                    let layer = silk_for(app.active_layer.get_untracked());
                    let w = self.rules(app, k).silk_width;
                    self.edit_pcb(app, k, "Draw line", false, |pcb, ids| {
                        pcb.graphics.push(PcbGraphic {
                            id: ids.fresh(),
                            layer,
                            shape: Shape::Polyline {
                                points: pts,
                                width: w,
                            },
                        })
                    });
                }
            }
            _ => {}
        }
    }

    fn tool_key(&self, app: &AppState, k: &Kind, v: &mut View, key: &str) -> bool {
        let routing = app.tool.get_untracked() == Tool::Track && !v.tool.points.is_empty();
        if !routing {
            return false;
        }
        match key {
            "/" => {
                v.tool.alt = !v.tool.alt;
                true
            }
            "V" => {
                // Commit up to the cursor, drop a via, continue on the other side.
                self.commit_route(app, k, v, false);
                let at = *v.tool.points.last().unwrap();
                let net = v.tool.net.clone();
                let (d, dr) = self.rules(app, k).via_for(&net);
                self.edit_pcb(app, k, "Place via", false, |pcb, ids| {
                    pcb.vias.push(Via {
                        id: ids.fresh(),
                        pos: at,
                        diameter: d,
                        drill: dr,
                        net,
                    })
                });
                let next = if app.active_layer.get_untracked() == Layer::TopCopper {
                    Layer::BottomCopper
                } else {
                    Layer::TopCopper
                };
                app.active_layer.set(next);
                app.status
                    .set(format!("Via placed; now routing on {}", next.name()));
                true
            }
            "Backspace" => {
                app.undo();
                false
            }
            "Escape" | "Enter" => {
                self.finish(app, k, v);
                true
            }
            _ => false,
        }
    }

    fn overlay(&self, app: &AppState, k: &Kind, v: &View, m: &mut Mesh) {
        let mut items = Vec::new();
        let layer = app.active_layer.get_untracked();
        let col = rgba(layer.color(), 200);
        match app.tool.get_untracked() {
            Tool::Track if !v.tool.points.is_empty() => {
                let (path, ok) = self.proposal(app, k, v);
                let c = if ok { col } else { rgba(0xff3030, 220) };
                items.push(Item {
                    prim: Prim::Path {
                        pts: path,
                        w: v.tool.width,
                        closed: false,
                    },
                    color: c,
                    owner: None,
                });
            }
            Tool::Track | Tool::Via => {
                let p = self
                    .anchor(app, k, v, layer)
                    .map(|a| a.0)
                    .unwrap_or(v.cursor);
                let r = self.rules(app, k);
                items.push(Item {
                    prim: Prim::Ring {
                        c: p,
                        r: r.via_diameter / 2,
                        w: 0,
                    },
                    color: col,
                    owner: None,
                });
            }
            Tool::Pour | Tool::Line if !v.tool.points.is_empty() => {
                let mut pts = v.tool.points.clone();
                pts.push(v.cursor);
                let closed = app.tool.get_untracked() == Tool::Pour;
                items.push(Item {
                    prim: Prim::Path { pts, w: 0, closed },
                    color: col,
                    owner: None,
                });
            }
            Tool::Outline if v.tool.points.len() == 1 => {
                let r = Rect::new(v.tool.points[0], v.cursor);
                items.push(Item {
                    prim: Prim::Path {
                        pts: r.corners().to_vec(),
                        w: 0,
                        closed: true,
                    },
                    color: rgba(Layer::Outline.color(), 255),
                    owner: None,
                });
                let label = format!("{} x {}", app.fmt_len(r.width()), app.fmt_len(r.height()));
                scene::shape_items(
                    &Shape::Text {
                        pos: r.max + Point::new(500_000, 500_000),
                        text: label,
                        size: (12.0 / v.camera.zoom * 1e6) as i64,
                        rotation: 0.0,
                        align: Align::Left,
                    },
                    rgba(0xffffff, 230),
                    None,
                    &mut items,
                );
            }
            Tool::Measure if !v.tool.points.is_empty() => {
                let a = v.tool.points[0];
                let b = if v.tool.points.len() > 1 {
                    v.tool.points[1]
                } else {
                    v.cursor
                };
                items.push(Item {
                    prim: Prim::Seg { a, b, w: 0 },
                    color: rgba(0xffe060, 255),
                    owner: None,
                });
                let d = a.dist(b) as i64;
                let dx = (b.x - a.x).abs();
                let dy = (b.y - a.y).abs();
                let label = format!(
                    "{}  (dx {}, dy {})",
                    app.fmt_len(d),
                    app.fmt_len(dx),
                    app.fmt_len(dy)
                );
                scene::shape_items(
                    &Shape::Text {
                        pos: b + Point::new(400_000, 400_000),
                        text: label,
                        size: (12.0 / v.camera.zoom * 1e6) as i64,
                        rotation: 0.0,
                        align: Align::Left,
                    },
                    rgba(0xffe060, 255),
                    None,
                    &mut items,
                );
            }
            _ => {}
        }
        // Rule-check markers. Unrouted nets already show as ratsnest lines,
        // so only errors get a ring.
        let r = (7.0 / v.camera.zoom * 1e6) as i64;
        for is in app.issues.get_untracked() {
            if let Some(p) = is.pos {
                if is.sheet.is_none() && is.severity == easypro_core::check::Severity::Error {
                    let c = rgba(0xff3b3b, 230);
                    items.push(Item {
                        prim: Prim::Ring {
                            c: p,
                            r,
                            w: (2.0 / v.camera.zoom * 1e6) as i64,
                        },
                        color: c,
                        owner: None,
                    });
                }
            }
        }
        draw_items(v, m, &items);
    }

    fn describe(&self, app: &AppState, k: &Kind, id: Id) -> String {
        app.project.with_untracked(|p| {
            let Some(b) = p.board(board(k)) else {
                return String::new();
            };
            let pcb = &b.pcb;
            if let Some(c) = pcb.component(id) {
                let fp = p
                    .library
                    .footprints
                    .get(&c.footprint)
                    .map(|f| f.name.as_str())
                    .unwrap_or("");
                return format!(
                    "{} · {} · {fp}{}",
                    c.designator,
                    c.value,
                    if c.locked { " · locked" } else { "" }
                );
            }
            if let Some(t) = pcb.tracks.iter().find(|t| t.id == id) {
                return format!(
                    "Track · {} · {} · {}",
                    if t.net.is_empty() { "no net" } else { &t.net },
                    app.fmt_len(t.width),
                    t.layer.name()
                );
            }
            if let Some(v) = pcb.vias.iter().find(|t| t.id == id) {
                return format!(
                    "Via · {} · {}/{}",
                    if v.net.is_empty() { "no net" } else { &v.net },
                    app.fmt_len(v.diameter),
                    app.fmt_len(v.drill)
                );
            }
            if let Some(po) = pcb.pours.iter().find(|t| t.id == id) {
                return format!("Copper area · {} · {}", po.net, po.layer.name());
            }
            String::new()
        })
    }

    fn open_item(&self, app: &AppState, k: &Kind, id: Id) {
        // Double-clicking copper highlights its net; parts open properties.
        let net = app.project.with_untracked(|p| {
            let b = p.board(board(k))?;
            b.pcb
                .tracks
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.net.clone())
                .or_else(|| {
                    b.pcb
                        .vias
                        .iter()
                        .find(|t| t.id == id)
                        .map(|t| t.net.clone())
                })
        });
        match net {
            Some(n) if !n.is_empty() => {
                app.highlight_net.set(Some(n.clone()));
                app.status
                    .set(format!("Highlighting net {n} (Esc to clear)"));
            }
            _ => {
                app.selection.set(vec![id]);
                app.show_right.set(true);
            }
        }
    }
}
