//! Schematic editor behaviour.

use super::canvas::{draw_items, Built, Handler, Kind, View};
use crate::state::{AppState, Tool};
use easypro_core::geom::{Point, Rect};
use easypro_core::id::Id;
use easypro_core::pick;
use easypro_core::project::Project;
use easypro_core::route::path90;
use easypro_core::scene::{self, rgba, Item, Prim, SceneLayer, SchTheme};
use easypro_core::schematic::{Annotation, Component, NetLabel, NetPort, NoConnect, PortStyle, Wire};
use easypro_core::shape::{Align, Shape};
use easypro_core::units::mil;
use easypro_render::gl::{GridStyle, LayerDraw};
use easypro_render::tess::Mesh;
use leptos::prelude::*;

pub struct SchHandler;

fn kk(k: &Kind) -> (Id, usize) {
    match k {
        Kind::Sch { board, sheet } => (*board, *sheet),
        _ => (Id(0), 0),
    }
}

pub fn theme(app: &AppState) -> SchTheme {
    if app.dark.get_untracked() { SchTheme::DARK } else { SchTheme::LIGHT }
}

fn prompt(msg: &str, default: &str) -> Option<String> {
    let r = web_sys::window()?.prompt_with_message_and_default(msg, default).ok()??;
    let t = r.trim().to_string();
    (!t.is_empty()).then_some(t)
}

/// Suggest the next label name: "D3" after "D2", otherwise "NET1".
fn next_label(app: &AppState, k: &Kind) -> String {
    let (b, s) = kk(k);
    let last = app.project.with_untracked(|p| p.board(b).and_then(|bd| bd.schematic.sheets.get(s)).and_then(|sh| sh.labels.last().map(|l| l.name.clone())));
    match last {
        Some(name) => {
            let digits: String = name.chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
            if digits.is_empty() {
                format!("{name}1")
            } else {
                let n: u64 = digits.parse().unwrap_or(0);
                format!("{}{}", &name[..name.len() - digits.len()], n + 1)
            }
        }
        None => "NET1".into(),
    }
}

impl SchHandler {
    fn edit_sheet(&self, app: &AppState, k: &Kind, label: &str, f: impl FnOnce(&mut easypro_core::schematic::Sheet, &mut easypro_core::id::IdGen)) {
        let (b, s) = kk(k);
        app.edit(label, |p| {
            let mut ids = std::mem::take(&mut p.ids);
            if let Some(sh) = p.board_mut(b).and_then(|bd| bd.schematic.sheets.get_mut(s)) {
                f(sh, &mut ids);
                sh.tidy_wires();
            }
            p.ids = ids;
        });
    }

    /// Snap to a nearby pin or wire end when enabled.
    fn snap_pin(&self, app: &AppState, k: &Kind, v: &View) -> Point {
        if !app.settings.with_untracked(|s| s.snap_to_pins) {
            return v.cursor;
        }
        let (b, s) = kk(k);
        let tol = v.tol() * 2;
        app.project
            .with_untracked(|p| p.board(b).and_then(|bd| bd.schematic.sheets.get(s)).and_then(|sh| pick::sch_pin_near(sh, &p.library, v.raw, tol)))
            .unwrap_or(v.cursor)
    }

    fn finish_wire(&self, app: &AppState, k: &Kind, v: &mut View) {
        let mut pts = std::mem::take(&mut v.tool.points);
        pts.dedup();
        if pts.len() >= 2 {
            let n = pts.len();
            self.edit_sheet(app, k, "Draw wire", |sh, ids| sh.wires.push(Wire { id: ids.fresh(), points: pts }));
            app.status.set(format!("Wire with {} segment(s) added", n - 1));
        }
        v.tool = Default::default();
    }

    fn ghost_component(&self, app: &AppState, device: &str, at: Point) -> Component {
        let prefix = app.project.with_untracked(|p| p.library.devices.get(device).map(|d| d.prefix.clone()).unwrap_or_else(|| "U".into()));
        let value = app.project.with_untracked(|p| p.library.devices.get(device).map(|d| d.value.clone()).unwrap_or_default());
        Component {
            id: Id(u64::MAX),
            device: device.to_string(),
            designator: format!("{prefix}?"),
            value,
            pos: at,
            rotation: app.place_rot.get_untracked(),
            mirror: app.place_mirror.get_untracked(),
            attributes: Default::default(),
            exclude_from_bom: false,
            designator_offset: None,
            value_offset: None,
        }
    }
}

impl Handler for SchHandler {
    fn build(&self, app: &AppState, k: &Kind) -> Option<Built> {
        let (b, s) = kk(k);
        let dark = app.dark.get_untracked();
        let th = theme(app);
        app.project.with_untracked(|p| {
            let bd = p.board(b)?;
            let sheet = bd.schematic.sheets.get(s)?;
            let conn = easypro_core::netlist::connectivity(&bd.schematic, &p.library);
            let scene = scene::sheet_scene(sheet, &p.library, Some((&conn, s)), &th, &p.name);
            let grid_mm = app.settings.with_untracked(|st| st.sch_grid) as f64 / 1e6;
            Some(Built {
                fit: sheet.frame().union(scene.bbox()),
                scene,
                background: if dark { [0.118, 0.122, 0.133] } else { [1.0, 1.0, 1.0] },
                grid: Some(GridStyle {
                    step_mm: grid_mm,
                    lines: app.settings.with_untracked(|st| st.grid_lines),
                    color: if dark { [0.3, 0.31, 0.34, 1.0] } else { [0.80, 0.82, 0.86, 1.0] },
                    major: if dark { [0.36, 0.37, 0.41, 1.0] } else { [0.70, 0.72, 0.77, 1.0] },
                }),
                order: vec![LayerDraw { key: SceneLayer::Frame, alpha: 1.0 }, LayerDraw { key: SceneLayer::Schematic, alpha: 1.0 }],
            })
        })
    }

    fn snap(&self, app: &AppState) -> i64 {
        app.settings.with_untracked(|s| s.sch_grid)
    }

    fn hit(&self, app: &AppState, k: &Kind, p: Point, tol: i64) -> Option<Id> {
        let (b, s) = kk(k);
        app.project.with_untracked(|pr| pr.board(b).and_then(|bd| bd.schematic.sheets.get(s)).and_then(|sh| pick::sch_hit(sh, &pr.library, p, tol)))
    }

    fn in_rect(&self, app: &AppState, k: &Kind, r: Rect, crossing: bool) -> Vec<Id> {
        let (b, s) = kk(k);
        app.project.with_untracked(|pr| pr.board(b).and_then(|bd| bd.schematic.sheets.get(s)).map(|sh| pick::sch_in_rect(sh, &pr.library, r, crossing)).unwrap_or_default())
    }

    fn apply_move(&self, p: &mut Project, k: &Kind, ids: &[Id], d: Point) {
        let (b, s) = kk(k);
        let lib = p.library.clone();
        if let Some(sh) = p.board_mut(b).and_then(|bd| bd.schematic.sheets.get_mut(s)) {
            easypro_core::edit::sch_move(sh, &lib, ids, d);
        }
    }

    fn tool_down(&self, app: &AppState, k: &Kind, v: &mut View, _ev: &web_sys::PointerEvent) -> bool {
        let at = v.cursor;
        match app.tool.get_untracked() {
            Tool::Wire => {
                let p = self.snap_pin(app, k, v);
                if v.tool.points.is_empty() {
                    v.tool.points.push(p);
                    return true;
                }
                let last = *v.tool.points.last().unwrap();
                if p == last {
                    self.finish_wire(app, k, v);
                    return true;
                }
                for q in path90(last, p, !v.tool.alt).into_iter().skip(1) {
                    v.tool.points.push(q);
                }
                // Ending on a pin or another wire finishes the wire.
                let (b, s) = kk(k);
                let tol = v.tol();
                let ends = app.project.with_untracked(|pr| {
                    pr.board(b).and_then(|bd| bd.schematic.sheets.get(s)).is_some_and(|sh| {
                        pick::sch_pin_near(sh, &pr.library, p, tol / 2).is_some()
                            || sh.wires.iter().any(|w| w.segments().any(|(a, c)| easypro_core::geom::point_on_segment(p, a, c)))
                    })
                });
                if ends && v.tool.points.len() > 2 {
                    self.finish_wire(app, k, v);
                }
                true
            }
            Tool::Label => {
                let p = self.snap_pin(app, k, v);
                let name = prompt("Net label name", &next_label(app, k));
                if let Some(name) = name {
                    self.edit_sheet(app, k, "Add net label", |sh, ids| sh.labels.push(NetLabel { id: ids.fresh(), name, pos: p, rotation: 0.0 }));
                }
                true
            }
            Tool::Power | Tool::Ground => {
                let ground = app.tool.get_untracked() == Tool::Ground;
                let p = self.snap_pin(app, k, v);
                let (name, style) = if ground { ("GND", PortStyle::Ground) } else { ("VCC", PortStyle::Power) };
                self.edit_sheet(app, k, "Add power flag", |sh, ids| sh.ports.push(NetPort { id: ids.fresh(), name: name.into(), pos: p, style, rotation: 0.0 }));
                true
            }
            Tool::NoConnect => {
                let p = self.snap_pin(app, k, v);
                self.edit_sheet(app, k, "Add no-connect", |sh, ids| sh.no_connects.push(NoConnect { id: ids.fresh(), pos: p }));
                true
            }
            Tool::Text => {
                if let Some(text) = prompt("Text", "Note") {
                    self.edit_sheet(app, k, "Add text", |sh, ids| {
                        sh.annotations.push(Annotation { id: ids.fresh(), shape: Shape::Text { pos: at, text, size: scene::LABEL_TEXT, rotation: 0.0, align: Align::Left } })
                    });
                }
                true
            }
            Tool::Place(device) => {
                let mut c = self.ghost_component(app, &device, at);
                let (b, s) = kk(k);
                let mut placed = String::new();
                app.edit("Place part", |p| {
                    c.id = p.next_id();
                    let lib = p.library.clone();
                    if let Some(bd) = p.board_mut(b) {
                        if let Some(sh) = bd.schematic.sheets.get_mut(s) {
                            sh.components.push(c.clone());
                        }
                        easypro_core::sync::annotate(bd, &lib, false);
                        placed = bd.schematic.find_component(c.id).map(|x| x.designator.clone()).unwrap_or_default();
                    }
                });
                app.status.set(format!("Placed {placed}. Click to place another, Esc to stop."));
                true
            }
            _ => false,
        }
    }

    fn tool_dbl(&self, app: &AppState, k: &Kind, v: &mut View) {
        if app.tool.get_untracked() == Tool::Wire {
            self.finish_wire(app, k, v);
        }
    }

    fn tool_key(&self, app: &AppState, k: &Kind, v: &mut View, key: &str) -> bool {
        if app.tool.get_untracked() != Tool::Wire || v.tool.points.is_empty() {
            return false;
        }
        match key {
            "/" => {
                v.tool.alt = !v.tool.alt;
                true
            }
            "Backspace" => {
                v.tool.points.pop();
                true
            }
            "Enter" => {
                self.finish_wire(app, k, v);
                true
            }
            "Escape" => {
                self.finish_wire(app, k, v);
                true
            }
            _ => false,
        }
    }

    fn overlay(&self, app: &AppState, k: &Kind, v: &View, m: &mut Mesh) {
        let th = theme(app);
        let mut items = Vec::new();
        match app.tool.get_untracked() {
            Tool::Wire => {
                if let Some(last) = v.tool.points.last() {
                    let mut pts = v.tool.points.clone();
                    let target = self.snap_pin(app, k, v);
                    pts.extend(path90(*last, target, !v.tool.alt).into_iter().skip(1));
                    items.push(Item { prim: Prim::Path { pts, w: 152_400, closed: false }, color: th.wire, owner: None });
                }
                let p = self.snap_pin(app, k, v);
                items.push(Item { prim: Prim::Ring { c: p, r: mil(25.0), w: 0 }, color: th.wire, owner: None });
            }
            Tool::Place(device) => {
                let c = self.ghost_component(app, &device, v.cursor);
                let lib = app.project.with_untracked(|p| p.library.clone());
                scene::component_items(&c, &lib, &th, &mut items);
                for it in &mut items {
                    it.color = (it.color & 0xffffff00) | 0xb0;
                }
            }
            Tool::Power | Tool::Ground | Tool::Label | Tool::NoConnect => {
                let p = self.snap_pin(app, k, v);
                items.push(Item { prim: Prim::Ring { c: p, r: mil(25.0), w: 0 }, color: rgba(0x3a90ff, 255), owner: None });
            }
            _ => {}
        }
        draw_items(v, m, &items);
    }

    fn open_item(&self, app: &AppState, k: &Kind, id: Id) {
        let (b, s) = kk(k);
        let label = app.project.with_untracked(|p| p.board(b).and_then(|bd| bd.schematic.sheets.get(s)).and_then(|sh| sh.labels.iter().find(|l| l.id == id).map(|l| l.name.clone())));
        if let Some(old) = label {
            if let Some(name) = prompt("Rename net label", &old) {
                self.edit_sheet(app, k, "Rename net label", |sh, _| {
                    if let Some(l) = sh.labels.iter_mut().find(|l| l.id == id) {
                        l.name = name;
                    }
                });
            }
            return;
        }
        app.selection.set(vec![id]);
        app.show_right.set(true);
    }
}
