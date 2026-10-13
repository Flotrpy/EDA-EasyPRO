//! Implementations of the commands in [`crate::commands`].

use crate::js;
use crate::state::{AppState, BottomTab, Clip, Dialog, LeftTab, Tab, Theme, Tool, ViewCmd};
use crate::storage;
use easypro_core::edit;
use easypro_core::geom::{Point, Rect};
use easypro_core::id::Id;
use easypro_core::layer::Layer;
use easypro_core::library::{Device, Footprint, Symbol};
use easypro_core::project::{Project, FILE_EXTENSION};
use easypro_core::units::{mil, mm, Unit};
use leptos::prelude::*;
use leptos::task::spawn_local;

// ------------------------------------------------------------ helpers

pub fn board_id(app: &AppState) -> Option<Id> {
    app.tab_untracked().board().or_else(|| {
        app.project
            .with_untracked(|p| p.boards.first().map(|b| b.id))
    })
}

fn file_base(app: &AppState) -> String {
    let name = app.project.with_untracked(|p| p.name.clone());
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() {
        "project".into()
    } else {
        s
    }
}

fn selection_center(rects: impl Iterator<Item = Rect>) -> Option<Point> {
    let r = rects.fold(Rect::EMPTY, |a, b| a.union(b));
    (!r.is_empty()).then(|| r.center())
}

// ------------------------------------------------------------ file

pub fn new_project(app: &AppState) {
    app.dialog.set(Some(Dialog::NewProject));
}

pub fn create_project(app: &AppState, name: &str) {
    let mut p = Project::new(if name.trim().is_empty() {
        "Untitled"
    } else {
        name.trim()
    });
    p.created = js::now_iso();
    let key = storage::new_key();
    app.load_project(p, key.clone());
    persist(app);
    app.ok(format!("Created project \"{}\"", name.trim()));
}

/// Save to this browser (IndexedDB) right away.
pub fn persist(app: &AppState) {
    let app = *app;
    let key = app.key.get_untracked();
    let p = app.project.get_untracked();
    spawn_local(async move {
        if let Err(e) = storage::save_local(&key, &p).await {
            app.warn(format!("Could not save to browser storage: {e}"));
        }
        app.recent.set(storage::list_local().await);
    });
}

pub fn open_project(app: &AppState) {
    let app = *app;
    spawn_local(async move {
        if let Some((name, text)) = storage::pick_file(".json,application/json").await {
            open_project_text(&app, &name, &text);
        }
    });
}

pub fn open_project_text(app: &AppState, name: &str, text: &str) {
    match Project::from_json(text) {
        Ok(p) => {
            let pname = p.name.clone();
            app.load_project(p, storage::new_key());
            persist(app);
            app.ok(format!("Opened {name} ({pname})"));
        }
        Err(e) => app.error(format!("{name}: {e}")),
    }
}

/// Open one of the bundled examples by its index in `examples::all()`.
pub fn open_example_at(app: &AppState, index: usize) {
    let Some((name, make)) = easypro_core::examples::all().into_iter().nth(index) else {
        return;
    };
    let mut p = make();
    p.created = js::now_iso();
    app.load_project(p, storage::new_key());
    persist(app);
    app.ok(format!(
        "Opened example: {name}. Try Design > Design Rule Check, or open the 3D view (Alt+3)."
    ));
}

pub fn open_example(app: &AppState) {
    open_example_at(app, 0);
}

pub fn open_example_regulator(app: &AppState) {
    open_example_at(app, 1);
}

pub fn open_recent(app: &AppState, key: String) {
    let app = *app;
    spawn_local(async move {
        match storage::load_local(&key).await {
            Ok(p) => {
                let name = p.name.clone();
                app.load_project(p, key);
                app.ok(format!("Opened {name}"));
            }
            Err(e) => app.error(e),
        }
    });
}

pub fn save(app: &AppState) {
    let app = *app;
    app.project.update_untracked(|p| p.modified = js::now_iso());
    let key = app.key.get_untracked();
    let p = app.project.get_untracked();
    let name = format!("{}.{FILE_EXTENSION}", file_base(&app));
    spawn_local(async move {
        let _ = storage::save_local(&key, &p).await;
        let _ = storage::snapshot(&key, &p, "Saved").await;
        app.recent.set(storage::list_local().await);
        match storage::save_text(&key, &name, &p.to_json()).await {
            Some(file) => {
                app.dirty.set(false);
                app.saved_name.set(Some(file.clone()));
                app.ok(format!("Saved {file} (and a version in this browser)"));
            }
            None => app.info("Save cancelled; the project is still kept in this browser."),
        }
    });
}

pub fn save_as(app: &AppState) {
    js::forget_handle(&app.key.get_untracked());
    save(app);
}

pub fn version_history(app: &AppState) {
    app.dialog.set(Some(Dialog::History));
}

pub fn rename_project(app: &AppState) {
    app.dialog.set(Some(Dialog::Rename));
}

fn unique_id(existing: impl Fn(&str) -> bool, base: &str) -> String {
    if !existing(base) {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|c| !existing(c))
        .unwrap()
}

pub fn import_kicad_footprint(app: &AppState) {
    let app = *app;
    spawn_local(async move {
        if let Some((name, text)) = storage::pick_file(".kicad_mod").await {
            import_footprint_text(&app, &name, &text);
        }
    });
}

pub fn import_footprint_text(app: &AppState, name: &str, text: &str) {
    match easypro_core::kicad::import_footprint(text) {
        Ok(mut f) => {
            f.id = app
                .project
                .with_untracked(|p| unique_id(|i| p.library.footprints.contains_key(i), &f.id));
            let id = f.id.clone();
            let pads = f.pads.len();
            app.edit("Import footprint", |p| p.library.add_footprint(f));
            app.ok(format!("Imported footprint from {name} ({pads} pads)"));
            app.open(Tab::Footprint { id });
        }
        Err(e) => app.error(format!("{name}: {e}")),
    }
}

pub fn import_kicad_symbols(app: &AppState) {
    let app = *app;
    spawn_local(async move {
        if let Some((name, text)) = storage::pick_file(".kicad_sym").await {
            import_symbols_text(&app, &name, &text);
        }
    });
}

pub fn import_symbols_text(app: &AppState, name: &str, text: &str) {
    match easypro_core::kicad::import_symbols(text) {
        Ok(syms) => {
            let n = syms.len();
            let mut first = None;
            app.edit("Import symbols", |p| {
                for mut s in syms {
                    s.id = unique_id(|i| p.library.symbols.contains_key(i), &s.id);
                    first.get_or_insert(s.id.clone());
                    p.library.add_symbol(s);
                }
            });
            app.ok(format!("Imported {n} symbol(s) from {name}"));
            if let Some(id) = first {
                app.open(Tab::Symbol { id });
            }
        }
        Err(e) => app.error(format!("{name}: {e}")),
    }
}

/// Open or import a dropped file by its extension.
pub fn open_dropped(app: &AppState, name: &str, text: &str) {
    let lower = name.to_lowercase();
    if lower.ends_with(".kicad_mod") {
        import_footprint_text(app, name, text);
    } else if lower.ends_with(".kicad_sym") {
        import_symbols_text(app, name, text);
    } else if lower.ends_with(".json") {
        open_project_text(app, name, text);
    } else {
        app.warn(format!(
            "{name}: drop an .epro.json project, a .kicad_mod footprint or a .kicad_sym library"
        ));
    }
}

pub fn start_page(app: &AppState) {
    app.open(Tab::Start);
}

// ------------------------------------------------------------ edit

pub fn undo(app: &AppState) {
    app.undo();
}

pub fn redo(app: &AppState) {
    app.redo();
}

pub fn undo_history(app: &AppState) {
    app.dialog.set(Some(Dialog::UndoHistory));
}

pub fn copy(app: &AppState) {
    let sel = app.selection.get_untracked();
    if sel.is_empty() {
        return;
    }
    let Some(b) = board_id(app) else { return };
    let clip = app.project.with_untracked(|p| {
        let board = p.board(b)?;
        let mut library = easypro_core::library::Library::default();
        Some(match app.tab_untracked() {
            Tab::Schematic { sheet, .. } => {
                let fragment = edit::sch_copy(board.schematic.sheets.get(sheet)?, &sel);
                for c in &fragment.components {
                    library.import_device(&p.library, &c.device);
                }
                Clip::Sch { fragment, library }
            }
            Tab::Pcb { .. } => {
                let fragment = edit::pcb_copy(&board.pcb, &sel);
                for c in &fragment.components {
                    library.import_device(&p.library, &c.device);
                    if let Some(f) = p.library.footprints.get(&c.footprint) {
                        library
                            .footprints
                            .entry(f.id.clone())
                            .or_insert_with(|| f.clone());
                    }
                }
                Clip::Pcb { fragment, library }
            }
            _ => return None,
        })
    });
    if let Some(c) = clip {
        js::copy_text(&serde_json::to_string(&c).unwrap_or_default());
        app.clipboard.set_value(Some(c));
        app.status.set(format!("Copied {} object(s)", sel.len()));
    }
}

pub fn cut(app: &AppState) {
    copy(app);
    delete(app);
}

pub fn paste(app: &AppState) {
    paste_offset(app, None);
}

/// Add parts that came with pasted content to the project library.
fn merge_library(p: &mut Project, lib: &easypro_core::library::Library) {
    for s in lib.symbols.values() {
        p.library
            .symbols
            .entry(s.id.clone())
            .or_insert_with(|| s.clone());
    }
    for f in lib.footprints.values() {
        p.library
            .footprints
            .entry(f.id.clone())
            .or_insert_with(|| f.clone());
    }
    for d in lib.devices.values() {
        p.library
            .devices
            .entry(d.id.clone())
            .or_insert_with(|| d.clone());
    }
}

fn paste_offset(app: &AppState, offset: Option<Point>) {
    let Some(clip) = app.clipboard.get_value() else {
        // Nothing copied in this tab: try the system clipboard, which may
        // hold content copied from another project or browser tab.
        let app = *app;
        spawn_local(async move {
            let text = wasm_bindgen_futures::JsFuture::from(js::read_clipboard())
                .await
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            match serde_json::from_str::<Clip>(&text) {
                Ok(c) => {
                    app.clipboard.set_value(Some(c));
                    paste_offset(&app, offset);
                }
                Err(_) => app.status.set("Clipboard is empty".into()),
            }
        });
        return;
    };
    let Some(b) = board_id(app) else { return };
    let tab = app.tab_untracked();
    let cursor = app.cursor.get_untracked();
    let mut new_ids = Vec::new();
    match (clip, tab) {
        (
            Clip::Sch {
                fragment: frag,
                library,
            },
            Tab::Schematic { sheet, .. },
        ) => {
            let grid = app.settings.with_untracked(|s| s.sch_grid);
            app.edit("Paste", |p| {
                merge_library(p, &library);
                let lib = p.library.clone();
                let bb = edit::frag_bbox(&frag, &lib);
                let d = offset.unwrap_or_else(|| match cursor {
                    Some(c) if !bb.is_empty() => (c - bb.center()).snap(grid),
                    _ => Point::new(mil(500.0), -mil(500.0)),
                });
                let mut ids = std::mem::take(&mut p.ids);
                if let Some(s) = p
                    .board_mut(b)
                    .and_then(|bd| bd.schematic.sheets.get_mut(sheet))
                {
                    new_ids = edit::sch_paste(s, &frag, d, &mut ids);
                }
                p.ids = ids;
                // Pasted parts get the next free designators straight away.
                if let Some(bd) = p.board_mut(b) {
                    easypro_core::sync::annotate(bd, &lib, false);
                }
            });
        }
        (
            Clip::Pcb {
                fragment: frag,
                library,
            },
            Tab::Pcb { .. },
        ) => {
            app.edit("Paste", |p| {
                merge_library(p, &library);
                let d = offset.unwrap_or(Point::new(mm(2.0), -mm(2.0)));
                let mut ids = std::mem::take(&mut p.ids);
                if let Some(bd) = p.board_mut(b) {
                    new_ids = edit::pcb_paste(&mut bd.pcb, &frag, d, &mut ids);
                }
                p.ids = ids;
            });
        }
        _ => {
            app.status
                .set("Clipboard content does not fit this editor".into());
            return;
        }
    }
    app.selection.set(new_ids);
}

pub fn duplicate(app: &AppState) {
    copy(app);
    let off = match app.tab_untracked() {
        Tab::Pcb { .. } => Point::new(mm(2.0), -mm(2.0)),
        _ => Point::new(mil(500.0), -mil(500.0)),
    };
    paste_offset(app, Some(off));
}

pub fn delete(app: &AppState) {
    match app.tab_untracked() {
        Tab::Symbol { id } => return crate::editor::symbol::delete_items(app, &id),
        Tab::Footprint { id } => return crate::editor::footprint::delete_items(app, &id),
        _ => {}
    }
    let sel = app.selection.get_untracked();
    if sel.is_empty() {
        return;
    }
    let Some(b) = board_id(app) else { return };
    let tab = app.tab_untracked();
    app.edit("Delete", |p| {
        let Some(bd) = p.board_mut(b) else { return };
        match tab {
            Tab::Schematic { sheet, .. } => {
                if let Some(s) = bd.schematic.sheets.get_mut(sheet) {
                    for id in &sel {
                        s.remove(*id);
                    }
                }
            }
            Tab::Pcb { .. } => {
                for id in &sel {
                    bd.pcb.remove(*id);
                }
            }
            _ => {}
        }
    });
    app.selection.set(vec![]);
    app.status.set(format!("Deleted {} object(s)", sel.len()));
}

pub fn select_all(app: &AppState) {
    let Some(b) = board_id(app) else { return };
    let ids = app.project.with_untracked(|p| {
        let Some(bd) = p.board(b) else { return vec![] };
        match app.tab_untracked() {
            Tab::Schematic { sheet, .. } => bd
                .schematic
                .sheets
                .get(sheet)
                .map(|s| {
                    easypro_core::pick::sch_in_rect(
                        s,
                        &p.library,
                        Rect::new(
                            Point::new(-i64::MAX / 4, -i64::MAX / 4),
                            Point::new(i64::MAX / 4, i64::MAX / 4),
                        ),
                        true,
                    )
                })
                .unwrap_or_default(),
            Tab::Pcb { .. } => easypro_core::pick::pcb_in_rect(
                &bd.pcb,
                &p.library,
                Rect::new(
                    Point::new(-i64::MAX / 4, -i64::MAX / 4),
                    Point::new(i64::MAX / 4, i64::MAX / 4),
                ),
                true,
            ),
            _ => vec![],
        }
    });
    app.selection.set(ids);
}

pub fn rotate(app: &AppState) {
    if matches!(app.tool.get_untracked(), Tool::Place(_) | Tool::Pin) {
        app.place_rot
            .update(|r| *r = easypro_core::geom::normalize_angle(*r + 90.0));
        return;
    }
    match app.tab_untracked() {
        Tab::Symbol { id } => return crate::editor::symbol::rotate_items(app, &id),
        Tab::Footprint { id } => return crate::editor::footprint::rotate_items(app, &id),
        _ => {}
    }
    transform_selection(app, "Rotate", |p, b, tab, sel| {
        let lib = p.library.clone();
        let bd = p.board_mut(b)?;
        match tab {
            Tab::Schematic { sheet, .. } => {
                let s = bd.schematic.sheets.get_mut(sheet)?;
                let grid = mil(50.0);
                let c = selection_center(
                    sel.iter()
                        .filter_map(|id| easypro_core::pick::sch_bbox(s, &lib, *id)),
                )?
                .snap(grid);
                edit::sch_rotate(s, sel, c, 90.0);
                // Keep pins on grid after rotating about a snapped centre.
                Some(())
            }
            Tab::Pcb { .. } => {
                let c = if sel.len() == 1 {
                    bd.pcb.component(sel[0]).map(|c| c.pos).or_else(|| {
                        selection_center(
                            sel.iter()
                                .filter_map(|id| easypro_core::pick::pcb_bbox(&bd.pcb, &lib, *id)),
                        )
                    })?
                } else {
                    selection_center(
                        sel.iter()
                            .filter_map(|id| easypro_core::pick::pcb_bbox(&bd.pcb, &lib, *id)),
                    )?
                };
                edit::pcb_rotate(&mut bd.pcb, sel, c, 90.0);
                Some(())
            }
            _ => None,
        }
    });
}

fn transform_selection(
    app: &AppState,
    label: &str,
    f: impl FnOnce(&mut Project, Id, Tab, &[Id]) -> Option<()>,
) {
    let sel = app.selection.get_untracked();
    if sel.is_empty() {
        return;
    }
    let Some(b) = board_id(app) else { return };
    let tab = app.tab_untracked();
    app.try_edit(label, |p| f(p, b, tab, &sel).is_some());
}

pub fn mirror(app: &AppState) {
    if matches!(app.tool.get_untracked(), Tool::Place(_)) {
        app.place_mirror.update(|m| *m = !*m);
        return;
    }
    transform_selection(app, "Mirror", |p, b, tab, sel| {
        let lib = p.library.clone();
        let Tab::Schematic { sheet, .. } = tab else {
            return None;
        };
        let s = p.board_mut(b)?.schematic.sheets.get_mut(sheet)?;
        let c = selection_center(
            sel.iter()
                .filter_map(|id| easypro_core::pick::sch_bbox(s, &lib, *id)),
        )?
        .snap(mil(50.0));
        edit::sch_mirror(s, sel, c);
        Some(())
    });
}

pub fn flip(app: &AppState) {
    transform_selection(app, "Flip side", |p, b, _, sel| {
        edit::pcb_flip(&mut p.board_mut(b)?.pcb, sel);
        Some(())
    });
}

/// Move the selection by whole grid steps.
fn nudge(app: &AppState, dx: i64, dy: i64) {
    use crate::editor::canvas::{handler, Kind};
    let Some(k) = Kind::from_tab(&app.tab_untracked()) else {
        return;
    };
    let sel = app.selection.get_untracked();
    if sel.is_empty() {
        return;
    }
    let h = handler(&k);
    let g = h.snap(app).max(1);
    app.edit("Nudge", |p| {
        h.apply_move(p, &k, &sel, Point::new(dx * g, dy * g))
    });
}

pub fn nudge_left(app: &AppState) {
    nudge(app, -1, 0);
}
pub fn nudge_right(app: &AppState) {
    nudge(app, 1, 0);
}
pub fn nudge_up(app: &AppState) {
    nudge(app, 0, 1);
}
pub fn nudge_down(app: &AppState) {
    nudge(app, 0, -1);
}
pub fn nudge_left10(app: &AppState) {
    nudge(app, -10, 0);
}
pub fn nudge_right10(app: &AppState) {
    nudge(app, 10, 0);
}
pub fn nudge_up10(app: &AppState) {
    nudge(app, 0, 10);
}
pub fn nudge_down10(app: &AppState) {
    nudge(app, 0, -10);
}

pub fn escape(app: &AppState) {
    if app.palette.get_untracked() {
        app.palette.set(false);
    } else if app.dialog.get_untracked().is_some() {
        app.dialog.set(None);
    } else if app.menu_open.get_untracked().is_some() || app.context_menu.get_untracked().is_some()
    {
        app.menu_open.set(None);
        app.context_menu.set(None);
    } else if app.tool.get_untracked() != Tool::Select {
        app.tool.set(Tool::Select);
        app.place_rot.set(0.0);
        app.place_mirror.set(false);
    } else if !app.selection.get_untracked().is_empty() {
        app.selection.set(vec![]);
    } else {
        app.highlight_net.set(None);
    }
}

// ------------------------------------------------------------ view

pub fn fit(app: &AppState) {
    app.view_cmd.set(Some(ViewCmd::Fit));
}

pub fn zoom_selection(app: &AppState) {
    let sel = app.selection.get_untracked();
    let Some(b) = board_id(app) else { return };
    let r = app.project.with_untracked(|p| {
        let bd = p.board(b)?;
        let rects: Vec<Rect> = match app.tab_untracked() {
            Tab::Schematic { sheet, .. } => {
                let s = bd.schematic.sheets.get(sheet)?;
                sel.iter()
                    .filter_map(|id| easypro_core::pick::sch_bbox(s, &p.library, *id))
                    .collect()
            }
            Tab::Pcb { .. } => sel
                .iter()
                .filter_map(|id| easypro_core::pick::pcb_bbox(&bd.pcb, &p.library, *id))
                .collect(),
            _ => vec![],
        };
        Some(rects.into_iter().fold(Rect::EMPTY, |a, b| a.union(b)))
    });
    match r {
        Some(r) if !r.is_empty() => app.view_cmd.set(Some(ViewCmd::FitRect(r))),
        _ => fit(app),
    }
}

pub fn zoom_in(app: &AppState) {
    app.view_cmd.set(Some(ViewCmd::ZoomIn));
}

pub fn zoom_out(app: &AppState) {
    app.view_cmd.set(Some(ViewCmd::ZoomOut));
}

pub fn toggle_left(app: &AppState) {
    app.show_left.update(|v| *v = !*v);
}

pub fn toggle_right(app: &AppState) {
    app.show_right.update(|v| *v = !*v);
}

pub fn toggle_bottom(app: &AppState) {
    app.show_bottom.update(|v| *v = !*v);
}

fn update_settings(app: &AppState, f: impl FnOnce(&mut crate::state::Settings)) {
    app.settings.update(f);
    app.settings.with_untracked(|s| s.save());
}

pub fn toggle_grid_style(app: &AppState) {
    update_settings(app, |s| s.grid_lines = !s.grid_lines);
}

pub fn toggle_units(app: &AppState) {
    update_settings(app, |s| {
        s.unit = if s.unit == Unit::Mm {
            Unit::Mil
        } else {
            Unit::Mm
        }
    });
    let u = app.settings.with_untracked(|s| s.unit);
    app.status.set(format!("Units: {}", u.label()));
}

pub fn toggle_theme(app: &AppState) {
    let dark = app.dark.get_untracked();
    update_settings(app, |s| {
        s.theme = if dark { Theme::Light } else { Theme::Dark }
    });
}

pub fn open_3d(app: &AppState) {
    if let Some(b) = board_id(app) {
        app.open(Tab::View3d { board: b });
    }
}

pub fn open_sch(app: &AppState) {
    if let Some(b) = board_id(app) {
        app.open(Tab::Schematic { board: b, sheet: 0 });
    }
}

pub fn open_pcb(app: &AppState) {
    if let Some(b) = board_id(app) {
        app.open(Tab::Pcb { board: b });
    }
}

pub fn focus_library(app: &AppState) {
    app.show_left.set(true);
    app.left_tab.set(LeftTab::Library);
    set_timeout_focus("library-search");
}

fn set_timeout_focus(id: &'static str) {
    set_timeout(
        move || {
            if let Some(el) = document().get_element_by_id(id) {
                use wasm_bindgen::JsCast;
                if let Ok(i) = el.dyn_into::<web_sys::HtmlElement>() {
                    let _ = i.focus();
                }
            }
        },
        std::time::Duration::from_millis(30),
    );
}

fn step_tab(app: &AppState, d: isize) {
    let n = app.tabs.with_untracked(|t| t.len()) as isize;
    let i = (app.active.get_untracked() as isize + d).rem_euclid(n.max(1)) as usize;
    let t = app.tabs.with_untracked(|t| t[i].clone());
    app.open(t);
}

pub fn next_tab(app: &AppState) {
    step_tab(app, 1);
}

pub fn prev_tab(app: &AppState) {
    step_tab(app, -1);
}

pub fn close_tab(app: &AppState) {
    app.close_tab(app.active.get_untracked());
}

pub fn palette(app: &AppState) {
    app.palette.set(true);
}

/// Look at the board from below; the active layer follows.
pub fn flip_view(app: &AppState) {
    let flipped = !app.pcb_flipped.get_untracked();
    app.pcb_flipped.set(flipped);
    app.active_layer.set(if flipped {
        Layer::BottomCopper
    } else {
        Layer::TopCopper
    });
    app.status.set(if flipped {
        "Viewing from the bottom (mirrored)".into()
    } else {
        "Viewing from the top".into()
    });
}

pub fn clear_highlight(app: &AppState) {
    app.highlight_net.set(None);
}

// ------------------------------------------------------------ tools

fn set_tool(app: &AppState, t: Tool) {
    app.place_rot.set(0.0);
    app.place_mirror.set(false);
    app.status.set(t.hint().to_string());
    app.tool.set(t);
}

pub fn tool_select(app: &AppState) {
    set_tool(app, Tool::Select);
}
pub fn tool_wire(app: &AppState) {
    set_tool(app, Tool::Wire);
}
pub fn tool_label(app: &AppState) {
    set_tool(app, Tool::Label);
}
pub fn tool_power(app: &AppState) {
    set_tool(app, Tool::Power);
}
pub fn tool_ground(app: &AppState) {
    set_tool(app, Tool::Ground);
}
pub fn tool_noconnect(app: &AppState) {
    set_tool(app, Tool::NoConnect);
}
pub fn tool_text(app: &AppState) {
    set_tool(app, Tool::Text);
}
pub fn tool_track(app: &AppState) {
    set_tool(app, Tool::Track);
}
pub fn tool_via(app: &AppState) {
    set_tool(app, Tool::Via);
}
pub fn tool_pour(app: &AppState) {
    set_tool(app, Tool::Pour);
}
pub fn tool_outline(app: &AppState) {
    set_tool(app, Tool::Outline);
}
pub fn tool_line(app: &AppState) {
    set_tool(app, Tool::Line);
}
pub fn tool_board_text(app: &AppState) {
    set_tool(app, Tool::BoardText);
}
pub fn tool_measure(app: &AppState) {
    set_tool(app, Tool::Measure);
}
pub fn tool_pin(app: &AppState) {
    set_tool(app, Tool::Pin);
}
pub fn tool_sym_line(app: &AppState) {
    set_tool(app, Tool::SymLine);
}
pub fn tool_sym_rect(app: &AppState) {
    set_tool(app, Tool::SymRect);
}
pub fn tool_sym_circle(app: &AppState) {
    set_tool(app, Tool::SymCircle);
}
pub fn tool_pad_smd(app: &AppState) {
    set_tool(app, Tool::PadSmd);
}
pub fn tool_pad_tht(app: &AppState) {
    set_tool(app, Tool::PadTht);
}
pub fn tool_fp_line(app: &AppState) {
    set_tool(app, Tool::FpLine);
}
pub fn tool_fp_rect(app: &AppState) {
    set_tool(app, Tool::FpRect);
}
pub fn tool_fp_circle(app: &AppState) {
    set_tool(app, Tool::FpCircle);
}

pub fn place_device(app: &AppState, device: &str) {
    storage::remember_part(device);
    app.recent_parts.set(storage::recent_parts());
    // Copy the part into the project library first so the file is self-contained.
    let lib = easypro_core::builtin::library();
    let in_project = app
        .project
        .with_untracked(|p| p.library.devices.contains_key(device));
    if !in_project {
        if !lib.devices.contains_key(device) {
            return;
        }
        app.edit("Add part to project library", |p| {
            p.library.import_device(lib, device);
        });
    }
    if !matches!(app.tab_untracked(), Tab::Schematic { .. }) {
        if let Some(b) = board_id(app) {
            app.open(Tab::Schematic { board: b, sheet: 0 });
        }
    }
    set_tool(app, Tool::Place(device.to_string()));
}

pub fn layer_top(app: &AppState) {
    app.active_layer.set(Layer::TopCopper);
}

pub fn layer_bottom(app: &AppState) {
    app.active_layer.set(Layer::BottomCopper);
}

pub fn layer_next(app: &AppState) {
    let Some(b) = board_id(app) else { return };
    let stack = app.project.with_untracked(|p| {
        p.board(b)
            .map(|bd| bd.pcb.copper_stack())
            .unwrap_or_default()
    });
    let cur = app.active_layer.get_untracked();
    let i = stack
        .iter()
        .position(|l| *l == cur)
        .map(|i| (i + 1) % stack.len())
        .unwrap_or(0);
    if let Some(l) = stack.get(i) {
        app.active_layer.set(*l);
        app.status.set(format!("Active layer: {}", l.name()));
    }
}

/// Select every track and via of the net under the selection.
pub fn select_net(app: &AppState) {
    let Some(b) = board_id(app) else { return };
    let sel = app.selection.get_untracked();
    let found = app.project.with_untracked(|p| {
        let bd = p.board(b)?;
        let pcb = &bd.pcb;
        let net = sel.iter().find_map(|id| {
            pcb.tracks
                .iter()
                .find(|t| t.id == *id)
                .map(|t| t.net.clone())
                .or_else(|| pcb.vias.iter().find(|v| v.id == *id).map(|v| v.net.clone()))
                .or_else(|| {
                    pcb.pours
                        .iter()
                        .find(|v| v.id == *id)
                        .map(|v| v.net.clone())
                })
        })?;
        if net.is_empty() {
            return None;
        }
        let ids: Vec<Id> = pcb
            .tracks
            .iter()
            .filter(|t| t.net == net)
            .map(|t| t.id)
            .chain(pcb.vias.iter().filter(|v| v.net == net).map(|v| v.id))
            .collect();
        Some((net, ids))
    });
    match found {
        Some((net, ids)) => {
            app.status
                .set(format!("Selected {} item(s) on {net}", ids.len()));
            app.highlight_net.set(Some(net));
            app.selection.set(ids);
        }
        None => app.status.set("Select a track or via first".into()),
    }
}

/// Lock the selected parts, or unlock them if they are all locked.
pub fn toggle_lock(app: &AppState) {
    let Some(b) = board_id(app) else { return };
    let sel = app.selection.get_untracked();
    let mut state = None;
    app.try_edit("Lock", |p| {
        let Some(bd) = p.board_mut(b) else {
            return false;
        };
        let parts: Vec<_> = bd
            .pcb
            .components
            .iter_mut()
            .filter(|c| sel.contains(&c.id))
            .collect();
        if parts.is_empty() {
            return false;
        }
        let lock = !parts.iter().all(|c| c.locked);
        for c in parts {
            c.locked = lock;
        }
        state = Some(lock);
        true
    });
    match state {
        Some(true) => app
            .status
            .set("Locked: these parts can't be moved by accident".into()),
        Some(false) => app.status.set("Unlocked".into()),
        None => app.status.set("Select parts to lock".into()),
    }
}

fn align(app: &AppState, mode: edit::AlignMode) {
    let Some(b) = board_id(app) else { return };
    let sel = app.selection.get_untracked();
    let mut moved = 0;
    app.try_edit("Align", |p| {
        let lib = p.library.clone();
        if let Some(bd) = p.board_mut(b) {
            moved = edit::pcb_align(&mut bd.pcb, &sel, mode);
            if moved > 0 {
                bd.pcb.rebuild_pours(&lib);
            }
        }
        moved > 0
    });
    if moved == 0 {
        app.status
            .set("Select two or more unlocked parts to align".into());
    }
}

pub fn align_left(app: &AppState) {
    align(app, edit::AlignMode::Left);
}
pub fn align_right(app: &AppState) {
    align(app, edit::AlignMode::Right);
}
pub fn align_top(app: &AppState) {
    align(app, edit::AlignMode::Top);
}
pub fn align_bottom(app: &AppState) {
    align(app, edit::AlignMode::Bottom);
}
pub fn distribute_x(app: &AppState) {
    align(app, edit::AlignMode::DistributeX);
}
pub fn distribute_y(app: &AppState) {
    align(app, edit::AlignMode::DistributeY);
}

pub fn rebuild_pours(app: &AppState) {
    let Some(b) = board_id(app) else { return };
    app.edit("Rebuild copper areas", |p| {
        let lib = p.library.clone();
        if let Some(bd) = p.board_mut(b) {
            bd.pcb.rebuild_pours(&lib);
        }
    });
    app.ok("Copper areas rebuilt");
}

// ------------------------------------------------------------ design

pub fn annotate(app: &AppState) {
    annotate_impl(app, false);
}

pub fn reannotate(app: &AppState) {
    annotate_impl(app, true);
}

fn annotate_impl(app: &AppState, reset: bool) {
    let Some(b) = board_id(app) else { return };
    let mut n = 0;
    app.try_edit("Annotate", |p| {
        let lib = p.library.clone();
        if let Some(bd) = p.board_mut(b) {
            n = easypro_core::sync::annotate(bd, &lib, reset);
        }
        n > 0
    });
    app.ok(if n == 0 {
        "All parts are already annotated".to_string()
    } else {
        format!("Annotated {n} part(s)")
    });
}

pub fn erc(app: &AppState) {
    let Some(b) = board_id(app) else { return };
    let issues = app.project.with_untracked(|p| {
        p.board(b)
            .map(|bd| easypro_core::erc::run(&bd.schematic, &p.library))
            .unwrap_or_default()
    });
    report_issues(app, "ERC", issues);
}

pub fn drc(app: &AppState) {
    let Some(b) = board_id(app) else { return };
    let unit = app.unit();
    let issues = app.project.with_untracked(|p| {
        p.board(b)
            .map(|bd| easypro_core::drc::run(&bd.pcb, &p.library, unit))
            .unwrap_or_default()
    });
    report_issues(app, "DRC", issues);
}

pub fn report_issues(app: &AppState, what: &str, issues: Vec<easypro_core::check::Issue>) {
    use easypro_core::check::Severity;
    let errors = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let warnings = issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .count();
    let summary = format!("{what}: {errors} error(s), {warnings} warning(s)");
    if errors > 0 {
        app.warn(summary.clone());
    } else {
        app.ok(if issues.is_empty() {
            format!("{what}: no problems found")
        } else {
            summary.clone()
        });
    }
    app.issues_title.set(summary);
    app.issues.set(issues);
    app.show_bottom.set(true);
    app.bottom_tab.set(BottomTab::Issues);
}

pub fn update_pcb(app: &AppState) {
    let Some(b) = board_id(app) else { return };
    let mut report = None;
    app.try_edit("Update PCB from schematic", |p| {
        let lib = p.library.clone();
        let mut ids = std::mem::take(&mut p.ids);
        if let Some(bd) = p.board_mut(b) {
            let r = easypro_core::sync::update_pcb(bd, &lib, &mut ids);
            if !r.is_empty() {
                bd.pcb.rebuild_pours(&lib);
            }
            report = Some(r);
        }
        p.ids = ids;
        report.as_ref().is_some_and(|r| !r.is_empty())
    });
    let Some(r) = report else { return };
    for w in &r.warnings {
        app.warn(w.clone());
    }
    for u in &r.updated {
        app.info(format!("Updated {u}"));
    }
    if r.is_empty() {
        app.ok("PCB is already up to date with the schematic");
    } else {
        app.ok(format!("PCB updated: {}", r.summary()));
        if !r.added.is_empty() {
            app.info("New footprints are placed to the right of the board. Drag them into place.");
        }
    }
    app.open(Tab::Pcb { board: b });
    app.view_cmd.set(Some(ViewCmd::Fit));
}

pub fn bom_dialog(app: &AppState) {
    app.dialog.set(Some(Dialog::Bom));
}

pub fn rules(app: &AppState) {
    app.dialog.set(Some(Dialog::Rules));
}

pub fn board_setup(app: &AppState) {
    app.dialog.set(Some(Dialog::BoardSetup));
}

pub fn add_sheet(app: &AppState) {
    let Some(b) = board_id(app) else { return };
    let mut idx = 0;
    app.edit("Add sheet", |p| {
        p.add_sheet(b);
        idx = p
            .board(b)
            .map(|bd| bd.schematic.sheets.len() - 1)
            .unwrap_or(0);
    });
    app.open(Tab::Schematic {
        board: b,
        sheet: idx,
    });
}

fn confirm(msg: &str) -> bool {
    web_sys::window()
        .and_then(|w| w.confirm_with_message(msg).ok())
        .unwrap_or(false)
}

pub fn remove_sheet(app: &AppState) {
    let Tab::Schematic { board, sheet } = app.tab_untracked() else {
        return;
    };
    if app.project.with_untracked(|p| {
        p.board(board)
            .map(|b| b.schematic.sheets.len())
            .unwrap_or(0)
    }) <= 1
    {
        app.warn("A board needs at least one sheet");
        return;
    }
    if !confirm("Delete this sheet and everything on it? You can undo with Ctrl+Z.") {
        return;
    }
    app.edit("Delete sheet", |p| {
        p.remove_sheet(board, sheet);
    });
    // Drop the sheet's tab and renumber later sheets.
    app.tabs.update(|tabs| {
        tabs.retain(|t| *t != Tab::Schematic { board, sheet });
        for t in tabs.iter_mut() {
            if let Tab::Schematic { board: b, sheet: s } = t {
                if *b == board && *s > sheet {
                    *s -= 1;
                }
            }
        }
    });
    app.active.set(0);
    app.open(Tab::Schematic {
        board,
        sheet: sheet.saturating_sub(1),
    });
}

pub fn remove_board(app: &AppState) {
    let Some(board) = app.tab_untracked().board() else {
        return;
    };
    if app.project.with_untracked(|p| p.boards.len()) <= 1 {
        app.warn("A project needs at least one board");
        return;
    }
    if !confirm("Delete this board with its schematic and PCB? You can undo with Ctrl+Z.") {
        return;
    }
    app.edit("Delete board", |p| {
        p.remove_board(board);
    });
    app.tabs
        .update(|tabs| tabs.retain(|t| t.board() != Some(board)));
    app.active.set(0);
    open_sch(app);
}

pub fn add_board(app: &AppState) {
    let mut id = None;
    app.edit("Add board", |p| {
        let n = p.boards.len() + 1;
        id = Some(p.add_board(&format!("Board{n}")));
    });
    if let Some(b) = id {
        app.open(Tab::Schematic { board: b, sheet: 0 });
    }
}

// ------------------------------------------------------------ tools menu

pub fn new_device(app: &AppState) {
    let mut id = String::new();
    app.edit("New device", |p| {
        let n = (1..)
            .find(|n| !p.library.devices.contains_key(&format!("dev:custom-{n}")))
            .unwrap();
        id = format!("dev:custom-{n}");
        let sym_id = format!("sym:custom-{n}");
        let fp_id = format!("fp:custom-{n}");
        let sym = easypro_core::builtin::symbols::ic_box(
            &sym_id,
            &format!("Custom {n}"),
            &[("1", "IN", easypro_core::library::PinKind::Input)],
            &[("2", "OUT", easypro_core::library::PinKind::Output)],
        );
        let mut fp = Footprint {
            id: fp_id.clone(),
            name: format!("Custom {n}"),
            ..Default::default()
        };
        fp.pads = easypro_core::builtin::footprints::sot23()
            .pads
            .into_iter()
            .take(2)
            .collect();
        p.library.add_symbol(Symbol { ..sym });
        p.library.add_footprint(fp);
        p.library.add_device(Device {
            id: id.clone(),
            name: format!("Custom Part {n}"),
            category: "Custom".into(),
            prefix: "U".into(),
            symbol: sym_id,
            footprint: Some(fp_id),
            value: format!("PART{n}"),
            ..Default::default()
        });
    });
    app.open(Tab::Device { id });
}

pub fn ic_wizard(app: &AppState) {
    app.dialog.set(Some(Dialog::IcWizard));
}

pub fn purge_library(app: &AppState) {
    let mut n = 0;
    app.try_edit("Remove unused parts", |p| {
        n = p.purge_unused_library();
        n > 0
    });
    app.ok(if n == 0 {
        "Every part in the project library is in use".to_string()
    } else {
        format!("Removed {n} unused library entries")
    });
}

pub fn validate_library(app: &AppState) {
    let problems = app.project.with_untracked(|p| p.library.validate());
    if problems.is_empty() {
        app.ok("Project library: every device has its symbol, footprint and pin mapping");
    } else {
        for pr in problems {
            app.warn(pr);
        }
    }
}

// ------------------------------------------------------------ export

pub fn fabrication(app: &AppState) {
    app.dialog.set(Some(Dialog::Fabrication));
}

fn with_board<T>(
    app: &AppState,
    f: impl FnOnce(&easypro_core::project::Board, &easypro_core::library::Library) -> T,
) -> Option<T> {
    let b = board_id(app)?;
    app.project
        .with_untracked(|p| p.board(b).map(|bd| f(bd, &p.library)))
}

pub fn export_bom(app: &AppState) {
    use easypro_export::bom::*;
    if let Some(csv) = with_board(app, |b, lib| {
        bom_csv(&lines(&b.schematic, lib), BomStyle::Generic)
    }) {
        js::download(
            &format!("{}-BOM.csv", file_base(app)),
            csv.as_bytes(),
            "text/csv",
        );
        app.ok("BOM exported");
    }
}

pub fn export_cpl(app: &AppState) {
    use easypro_export::bom::*;
    if let Some(csv) = with_board(app, |b, lib| cpl_csv(&b.pcb, lib, BomStyle::Generic)) {
        js::download(
            &format!("{}-CPL.csv", file_base(app)),
            csv.as_bytes(),
            "text/csv",
        );
        app.ok("Pick-and-place file exported");
    }
}

pub fn export_netlist(app: &AppState) {
    let base = file_base(app);
    if let Some(net) = with_board(app, |b, lib| {
        easypro_export::netlist::kicad_netlist(&b.schematic, lib, &base)
    }) {
        js::download(&format!("{base}.net"), net.as_bytes(), "text/plain");
        app.ok("KiCad netlist exported");
    }
}

pub fn export_csv_netlist(app: &AppState) {
    if let Some(csv) = with_board(app, |b, lib| {
        easypro_export::netlist::csv_netlist(&b.schematic, lib)
    }) {
        js::download(
            &format!("{}-netlist.csv", file_base(app)),
            csv.as_bytes(),
            "text/csv",
        );
        app.ok("CSV netlist exported");
    }
}

pub fn export_svg(app: &AppState) {
    let base = file_base(app);
    let dark = app.dark.get_untracked();
    let hidden = app.hidden_layers.get_untracked();
    let svg = app.project.with_untracked(|p| -> Option<(String, String)> {
        use easypro_core::scene::*;
        use easypro_export::svg::scene_to_svg;
        match app.tab_untracked() {
            Tab::Schematic { board, sheet } => {
                let bd = p.board(board)?;
                let s = bd.schematic.sheets.get(sheet)?;
                let conn = easypro_core::netlist::connectivity(&bd.schematic, &p.library);
                let theme = if dark {
                    SchTheme::DARK
                } else {
                    SchTheme::LIGHT
                };
                let scene = sheet_scene(s, &p.library, Some((&conn, sheet)), &theme, &p.name);
                Some((
                    format!("{base}-{}.svg", s.name),
                    scene_to_svg(&scene, Some(if dark { 0x1e1f22 } else { 0xffffff }), 0),
                ))
            }
            Tab::Pcb { board } => {
                let bd = p.board(board)?;
                let style = PcbStyle {
                    hidden,
                    highlight_net: None,
                    show_ratsnest: false,
                };
                let scene = pcb_scene(&bd.pcb, &p.library, &style, &[]);
                Some((
                    format!("{base}-pcb.svg"),
                    scene_to_svg(&scene, Some(0x101010), mm(2.0)),
                ))
            }
            Tab::Footprint { id } => {
                let f = p.library.footprints.get(&id)?;
                Some((
                    format!("{}.svg", f.name),
                    scene_to_svg(&footprint_scene(f), Some(0x101010), mm(1.0)),
                ))
            }
            Tab::Symbol { id } => {
                let s = p.library.symbols.get(&id)?;
                Some((
                    format!("{}.svg", s.name),
                    scene_to_svg(&symbol_scene(s, &SchTheme::LIGHT), Some(0xffffff), mm(2.0)),
                ))
            }
            _ => None,
        }
    });
    if let Some((name, text)) = svg {
        js::download(&name, text.as_bytes(), "image/svg+xml");
        app.ok(format!("Exported {name}"));
    }
}

/// Every sheet of the board on its own landscape page, via the browser's
/// print dialog (which can also save a PDF).
pub fn print_schematic(app: &AppState) {
    use easypro_core::scene::{sheet_scene, SchTheme};
    let Some(b) = board_id(app) else { return };
    let pages = app.project.with_untracked(|p| {
        let bd = p.board(b)?;
        let conn = easypro_core::netlist::connectivity(&bd.schematic, &p.library);
        Some(
            bd.schematic
                .sheets
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let scene =
                        sheet_scene(s, &p.library, Some((&conn, i)), &SchTheme::LIGHT, &p.name);
                    // Fit the frame exactly so each sheet fills its page.
                    let svg = easypro_export::svg::scene_to_svg(&scene, Some(0xffffff), 0);
                    (s.paper.label().to_string(), svg)
                })
                .collect::<Vec<_>>(),
        )
    });
    let Some(pages) = pages else { return };
    let title = app.project.with_untracked(|p| p.name.clone());
    let paper = pages
        .first()
        .map(|p| p.0.clone())
        .unwrap_or_else(|| "A4".into());
    let body: String = pages
        .into_iter()
        .map(|(_, svg)| format!("<section>{svg}</section>"))
        .collect();
    let html = format!(
        "<!doctype html><html><head><meta charset=utf-8><title>{title}</title><style>@page {{ size: {paper} landscape; margin: 6mm; }} body {{ margin: 0; }} section {{ page-break-after: always; height: 100vh; display: flex; align-items: center; justify-content: center; }} section svg {{ max-width: 100%; max-height: 100%; width: auto; height: auto; }}</style></head><body>{body}</body></html>"
    );
    if !js::print_html(&html) {
        app.warn(
            "The browser blocked the print window. Allow pop-ups for this site and try again.",
        );
    }
}

pub fn export_assembly(app: &AppState) {
    use easypro_core::layer::Side;
    let base = file_base(app);
    let files = with_board(app, |b, lib| {
        [Side::Top, Side::Bottom]
            .into_iter()
            .filter(|s| *s == Side::Top || b.pcb.components.iter().any(|c| c.side == *s))
            .map(|s| (s, easypro_export::assembly::assembly_svg(&b.pcb, lib, s)))
            .collect::<Vec<_>>()
    });
    for (side, svg) in files.unwrap_or_default() {
        let name = format!(
            "{base}-assembly-{}.svg",
            if side == Side::Top { "top" } else { "bottom" }
        );
        js::download(&name, svg.as_bytes(), "image/svg+xml");
        app.ok(format!("Exported {name}"));
    }
}

pub fn export_json(app: &AppState) {
    let json = app.project.with_untracked(|p| p.to_json());
    js::download(
        &format!("{}.{FILE_EXTENSION}", file_base(app)),
        json.as_bytes(),
        "application/json",
    );
}

// ------------------------------------------------------------ settings & help

pub fn preferences(app: &AppState) {
    app.dialog.set(Some(Dialog::Settings));
}

pub fn shortcuts(app: &AppState) {
    app.dialog.set(Some(Dialog::Shortcuts));
}

fn open_url(url: &str) {
    if let Some(w) = web_sys::window() {
        let _ = w.open_with_url_and_target(url, "_blank");
    }
}

pub fn open_guide(_app: &AppState) {
    open_url("https://github.com/Flotrpy/EDA-EasyPRO/blob/main/docs/USER_GUIDE.md");
}

pub fn report_issue(_app: &AppState) {
    open_url("https://github.com/Flotrpy/EDA-EasyPRO/issues/new");
}

pub fn about(app: &AppState) {
    app.dialog.set(Some(Dialog::About));
}
