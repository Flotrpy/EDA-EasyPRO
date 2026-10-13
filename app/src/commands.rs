//! Every user action in one table. Menus, the command palette and
//! keyboard shortcuts are all generated from it, so they never disagree.

use crate::actions as a;
use crate::state::{AppState, Tab};
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ctx {
    /// Always available.
    Any,
    /// A schematic tab is active.
    Sch,
    /// A PCB tab is active.
    Pcb,
    /// Schematic or PCB.
    Doc,
    /// Any 2D editor (schematic, PCB, symbol, footprint).
    Canvas,
    /// Symbol editor.
    Sym,
    /// Footprint editor.
    Fp,
}

pub struct Command {
    pub id: &'static str,
    pub label: &'static str,
    pub menu: &'static str,
    pub key: &'static str,
    pub ctx: Ctx,
    pub icon: &'static str,
    pub run: fn(&AppState),
}

macro_rules! cmd {
    ($id:expr, $label:expr, $menu:expr, $key:expr, $ctx:ident, $icon:expr, $run:expr) => {
        Command {
            id: $id,
            label: $label,
            menu: $menu,
            key: $key,
            ctx: Ctx::$ctx,
            icon: $icon,
            run: $run,
        }
    };
}

pub const MENUS: [&str; 10] = [
    "File", "Edit", "View", "Place", "Route", "Design", "Tools", "Export", "Settings", "Help",
];

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    // File
    cmd!("file.new", "New Project…", "File", "Ctrl+Alt+N", Any, "new", a::new_project),
    cmd!("file.open", "Open Project…", "File", "Ctrl+O", Any, "folder-open", a::open_project),
    cmd!("file.example", "Open Example: 555 LED Blinker", "File", "", Any, "book", a::open_example),
    cmd!("file.save", "Save", "File", "Ctrl+S", Any, "save", a::save),
    cmd!("file.save_as", "Save As…", "File", "Ctrl+Shift+S", Any, "save", a::save_as),
    cmd!("file.history", "Version History…", "File", "", Any, "history", a::version_history),
    cmd!("file.rename", "Rename Project…", "File", "", Any, "text", a::rename_project),
    cmd!("file.import_fp", "Import KiCad Footprint (.kicad_mod)…", "File", "", Any, "import", a::import_kicad_footprint),
    cmd!("file.import_sym", "Import KiCad Symbols (.kicad_sym)…", "File", "", Any, "import", a::import_kicad_symbols),
    cmd!("file.start", "Start Page", "File", "", Any, "list", a::start_page),
    // Edit
    cmd!("edit.undo", "Undo", "Edit", "Ctrl+Z", Any, "undo", a::undo),
    cmd!("edit.redo", "Redo", "Edit", "Ctrl+Y", Any, "redo", a::redo),
    cmd!("edit.redo2", "Redo", "", "Ctrl+Shift+Z", Any, "redo", a::redo),
    cmd!("edit.history", "Undo History…", "Edit", "", Any, "history", a::undo_history),
    cmd!("edit.cut", "Cut", "Edit", "Ctrl+X", Doc, "copy", a::cut),
    cmd!("edit.copy", "Copy", "Edit", "Ctrl+C", Doc, "copy", a::copy),
    cmd!("edit.paste", "Paste", "Edit", "Ctrl+V", Doc, "paste", a::paste),
    cmd!("edit.duplicate", "Duplicate", "Edit", "Ctrl+D", Doc, "copy", a::duplicate),
    cmd!("edit.delete", "Delete", "Edit", "Delete", Canvas, "trash", a::delete),
    cmd!("edit.delete2", "Delete", "", "Backspace", Canvas, "trash", a::delete),
    cmd!("edit.select_all", "Select All", "Edit", "Ctrl+A", Canvas, "select", a::select_all),
    cmd!("edit.rotate", "Rotate 90°", "Edit", "R", Canvas, "rotate", a::rotate),
    cmd!("edit.rotate_space", "Rotate 90°", "", "Space", Canvas, "rotate", a::rotate),
    cmd!("edit.mirror", "Mirror", "Edit", "X", Sch, "mirror", a::mirror),
    cmd!("edit.flip", "Flip to Other Side", "Edit", "F", Pcb, "flip", a::flip),
    cmd!("edit.nudge_left", "Nudge Left", "Edit", "ArrowLeft", Canvas, "chevron-right", a::nudge_left),
    cmd!("edit.nudge_right", "Nudge Right", "Edit", "ArrowRight", Canvas, "chevron-right", a::nudge_right),
    cmd!("edit.nudge_up", "Nudge Up", "Edit", "ArrowUp", Canvas, "chevron-right", a::nudge_up),
    cmd!("edit.nudge_down", "Nudge Down", "Edit", "ArrowDown", Canvas, "chevron-right", a::nudge_down),
    cmd!("edit.nudge_left10", "Nudge Left ×10", "", "Shift+ArrowLeft", Canvas, "chevron-right", a::nudge_left10),
    cmd!("edit.nudge_right10", "Nudge Right ×10", "", "Shift+ArrowRight", Canvas, "chevron-right", a::nudge_right10),
    cmd!("edit.nudge_up10", "Nudge Up ×10", "", "Shift+ArrowUp", Canvas, "chevron-right", a::nudge_up10),
    cmd!("edit.nudge_down10", "Nudge Down ×10", "", "Shift+ArrowDown", Canvas, "chevron-right", a::nudge_down10),
    cmd!("edit.escape", "Cancel / Select Tool", "", "Escape", Any, "select", a::escape),
    // View
    cmd!("view.fit", "Fit All", "View", "Home", Canvas, "fit", a::fit),
    cmd!("view.fit2", "Fit All", "", "Ctrl+0", Canvas, "fit", a::fit),
    cmd!("view.zoom_sel", "Zoom to Selection", "View", "Z", Canvas, "zoom-in", a::zoom_selection),
    cmd!("view.zoom_in", "Zoom In", "View", "=", Canvas, "zoom-in", a::zoom_in),
    cmd!("view.zoom_out", "Zoom Out", "View", "-", Canvas, "zoom-out", a::zoom_out),
    cmd!("view.left", "Toggle Left Panel", "View", "Ctrl+B", Any, "panel-left", a::toggle_left),
    cmd!("view.right", "Toggle Properties Panel", "View", "Ctrl+Alt+P", Any, "panel-right", a::toggle_right),
    cmd!("view.bottom", "Toggle Bottom Panel", "View", "Ctrl+J", Any, "panel-bottom", a::toggle_bottom),
    cmd!("view.grid", "Toggle Grid Dots/Lines", "View", "", Canvas, "grid", a::toggle_grid_style),
    cmd!("view.units", "Toggle Units (mm/mil)", "View", "Q", Pcb, "measure", a::toggle_units),
    cmd!("view.theme", "Toggle Dark Theme", "View", "", Any, "moon", a::toggle_theme),
    cmd!("view.3d", "3D View", "View", "Alt+3", Doc, "cube", a::open_3d),
    cmd!("view.sch", "Go to Schematic", "View", "Alt+1", Doc, "schematic", a::open_sch),
    cmd!("view.pcb", "Go to PCB", "View", "Alt+2", Doc, "pcb", a::open_pcb),
    cmd!("view.library", "Component Library", "View", "Shift+F", Any, "book", a::focus_library),
    cmd!("view.palette", "Command Palette…", "View", "Ctrl+K", Any, "search", a::palette),
    cmd!("view.palette2", "Command Palette…", "", "Ctrl+Shift+P", Any, "search", a::palette),
    cmd!("view.flip_board", "View from Bottom", "View", "Alt+F", Pcb, "flip", a::flip_view),
    cmd!("view.clear_highlight", "Clear Net Highlight", "View", "", Pcb, "eye", a::clear_highlight),
    // Place (schematic)
    cmd!("tool.select", "Select", "Place", "S", Canvas, "select", a::tool_select),
    cmd!("tool.wire", "Wire", "Place", "W", Sch, "wire", a::tool_wire),
    cmd!("tool.label", "Net Label", "Place", "N", Sch, "label", a::tool_label),
    cmd!("tool.power", "Power Flag", "Place", "P", Sch, "power", a::tool_power),
    cmd!("tool.ground", "Ground Flag", "Place", "G", Sch, "ground", a::tool_ground),
    cmd!("tool.noconnect", "No-Connect Flag", "Place", "Shift+X", Sch, "noconnect", a::tool_noconnect),
    cmd!("tool.text", "Text Note", "Place", "T", Sch, "text", a::tool_text),
    cmd!("tool.part", "Part from Library…", "Place", "", Sch, "chip", a::focus_library),
    // Route (PCB)
    cmd!("tool.track", "Track", "Route", "W", Pcb, "track", a::tool_track),
    cmd!("tool.via", "Via", "Route", "V", Pcb, "via", a::tool_via),
    cmd!("tool.pour", "Copper Area", "Route", "E", Pcb, "pour", a::tool_pour),
    cmd!("tool.outline", "Board Outline", "Route", "", Pcb, "outline", a::tool_outline),
    cmd!("tool.line", "Silkscreen Line", "Route", "L", Pcb, "line", a::tool_line),
    cmd!("tool.board_text", "Silkscreen Text", "Route", "Shift+T", Pcb, "text", a::tool_board_text),
    cmd!("tool.measure", "Measure", "Route", "M", Pcb, "measure", a::tool_measure),
    cmd!("pcb.layer_top", "Active Layer: Top", "Route", "T", Pcb, "layers", a::layer_top),
    cmd!("pcb.layer_bottom", "Active Layer: Bottom", "Route", "B", Pcb, "layers", a::layer_bottom),
    cmd!("pcb.layer_next", "Next Copper Layer", "Route", "Ctrl+Alt+L", Pcb, "layers", a::layer_next),
    cmd!("pcb.select_net", "Select Whole Net", "Route", "Alt+N", Pcb, "list", a::select_net),
    cmd!("pcb.lock", "Lock / Unlock Selection", "Route", "Shift+L", Pcb, "lock", a::toggle_lock),
    cmd!("pcb.align_left", "Align Left", "Route", "", Pcb, "list", a::align_left),
    cmd!("pcb.align_right", "Align Right", "Route", "", Pcb, "list", a::align_right),
    cmd!("pcb.align_top", "Align Top", "Route", "", Pcb, "list", a::align_top),
    cmd!("pcb.align_bottom", "Align Bottom", "Route", "", Pcb, "list", a::align_bottom),
    cmd!("pcb.distribute_x", "Distribute Horizontally", "Route", "", Pcb, "list", a::distribute_x),
    cmd!("pcb.distribute_y", "Distribute Vertically", "Route", "", Pcb, "list", a::distribute_y),
    cmd!("pcb.rebuild", "Rebuild Copper Areas", "Route", "Shift+B", Pcb, "pour", a::rebuild_pours),
    // Symbol / footprint editor tools
    cmd!("sym.pin", "Pin", "Place", "P", Sym, "pin", a::tool_pin),
    cmd!("sym.line", "Line", "Place", "L", Sym, "line", a::tool_sym_line),
    cmd!("sym.rect", "Rectangle", "Place", "", Sym, "rect", a::tool_sym_rect),
    cmd!("sym.circle", "Circle", "Place", "", Sym, "circle", a::tool_sym_circle),
    cmd!("fp.smd", "SMD Pad", "Place", "P", Fp, "pad", a::tool_pad_smd),
    cmd!("fp.tht", "Through-Hole Pad", "Place", "H", Fp, "tht", a::tool_pad_tht),
    cmd!("fp.line", "Silkscreen Line", "Place", "L", Fp, "line", a::tool_fp_line),
    cmd!("fp.rect", "Silkscreen Rectangle", "Place", "", Fp, "rect", a::tool_fp_rect),
    cmd!("fp.circle", "Silkscreen Circle", "Place", "", Fp, "circle", a::tool_fp_circle),
    // Design
    cmd!("design.annotate", "Annotate Designators", "Design", "", Sch, "number", a::annotate),
    cmd!("design.reannotate", "Re-annotate All", "Design", "", Sch, "number", a::reannotate),
    cmd!("design.erc", "Electrical Rule Check", "Design", "Ctrl+Alt+E", Sch, "check", a::erc),
    cmd!("design.update_pcb", "Update PCB from Schematic", "Design", "Ctrl+U", Doc, "sync", a::update_pcb),
    cmd!("design.drc", "Design Rule Check", "Design", "Ctrl+Alt+D", Pcb, "drc", a::drc),
    cmd!("design.bom", "Bill of Materials…", "Design", "", Doc, "list", a::bom_dialog),
    cmd!("design.rules", "Design Rules…", "Design", "", Pcb, "settings", a::rules),
    cmd!("design.board", "Board Setup (Layers, Thickness, Colour)…", "Design", "", Pcb, "board", a::board_setup),
    cmd!("design.add_sheet", "Add Schematic Sheet", "Design", "", Doc, "sheet", a::add_sheet),
    cmd!("design.remove_sheet", "Delete Current Sheet", "Design", "", Sch, "trash", a::remove_sheet),
    cmd!("design.add_board", "Add Board", "Design", "", Any, "board", a::add_board),
    cmd!("design.remove_board", "Delete Current Board", "Design", "", Doc, "trash", a::remove_board),
    // Tools
    cmd!("tools.new_device", "New Device…", "Tools", "", Any, "chip", a::new_device),
    cmd!("tools.ic_wizard", "IC Symbol Wizard…", "Tools", "", Any, "wand", a::ic_wizard),
    cmd!("tools.validate_lib", "Check Project Library", "Tools", "", Any, "check", a::validate_library),
    // Export
    cmd!("export.fab", "Fabrication Package (Gerber, Drill, BOM, CPL)…", "Export", "Ctrl+Alt+G", Doc, "export", a::fabrication),
    cmd!("export.bom", "BOM (CSV)", "Export", "", Doc, "list", a::export_bom),
    cmd!("export.cpl", "Pick and Place (CSV)", "Export", "", Doc, "cpl", a::export_cpl),
    cmd!("export.netlist", "KiCad Netlist (.net)", "Export", "", Doc, "schematic", a::export_netlist),
    cmd!("export.assembly", "Assembly Drawings (SVG, top and bottom)", "Export", "", Doc, "board", a::export_assembly),
    cmd!("export.svg", "SVG of Current View", "Export", "", Canvas, "download", a::export_svg),
    cmd!("export.print", "Print Schematic / Save as PDF…", "Export", "Ctrl+P", Doc, "sheet", a::print_schematic),
    cmd!("export.json", "Project File (.epro.json)", "Export", "", Any, "download", a::export_json),
    // Settings
    cmd!("settings.prefs", "Preferences…", "Settings", "Ctrl+,", Any, "settings", a::preferences),
    cmd!("settings.keys", "Keyboard Shortcuts…", "Settings", "", Any, "keyboard", a::shortcuts),
    // Help
    cmd!("help.shortcuts", "Shortcut Reference", "Help", "?", Any, "keyboard", a::shortcuts),
    cmd!("help.about", "About EasyPRO", "Help", "", Any, "info", a::about),
];

pub fn find(id: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.id == id)
}

/// Is the command usable for the current tab?
pub fn enabled(app: &AppState, c: &Command) -> bool {
    let tab = app.tab_untracked();
    match c.ctx {
        Ctx::Any => true,
        Ctx::Sch => matches!(tab, Tab::Schematic { .. }),
        Ctx::Pcb => matches!(tab, Tab::Pcb { .. }),
        Ctx::Doc => tab.board().is_some(),
        Ctx::Canvas => matches!(
            tab,
            Tab::Schematic { .. } | Tab::Pcb { .. } | Tab::Symbol { .. } | Tab::Footprint { .. }
        ),
        Ctx::Sym => matches!(tab, Tab::Symbol { .. }),
        Ctx::Fp => matches!(tab, Tab::Footprint { .. }),
    }
}

/// Effective key chord (user override or default).
pub fn key_for(app: &AppState, c: &Command) -> String {
    app.settings
        .with_untracked(|s| s.hotkeys.get(c.id).cloned())
        .unwrap_or_else(|| c.key.to_string())
}

pub fn run(app: &AppState, id: &str) {
    if let Some(c) = find(id) {
        if enabled(app, c) {
            app.menu_open.set(None);
            app.context_menu.set(None);
            (c.run)(app);
        } else {
            app.status
                .set(format!("\"{}\" is not available here", c.label));
        }
    }
}

/// Canonical chord text for a key event, e.g. `Ctrl+Shift+Z`.
pub fn chord(ev: &web_sys::KeyboardEvent) -> String {
    let mut k = ev.key();
    if k == " " {
        k = "Space".into();
    }
    if k.len() == 1 {
        k = k.to_uppercase();
    }
    if matches!(k.as_str(), "Control" | "Shift" | "Alt" | "Meta") {
        return String::new();
    }
    // Shift changes the key for symbols ("?" vs "/"); keep the produced key.
    let shift = ev.shift_key() && !(k.len() == 1 && !k.chars().all(|c| c.is_ascii_alphanumeric()));
    let mut s = String::new();
    if ev.ctrl_key() || ev.meta_key() {
        s.push_str("Ctrl+");
    }
    if ev.alt_key() {
        s.push_str("Alt+");
    }
    if shift {
        s.push_str("Shift+");
    }
    s.push_str(&k);
    s
}

/// Dispatch a key press. Returns true when a command handled it.
pub fn handle_key(app: &AppState, ev: &web_sys::KeyboardEvent) -> bool {
    let ch = chord(ev);
    if ch.is_empty() {
        return false;
    }
    // Prefer the most specific context when several commands share a key.
    let mut best: Option<&Command> = None;
    for c in COMMANDS.iter() {
        if key_for(app, c).eq_ignore_ascii_case(&ch) && enabled(app, c) {
            let specific = !matches!(c.ctx, Ctx::Any);
            if best.is_none() || (specific && matches!(best.unwrap().ctx, Ctx::Any)) {
                best = Some(c);
            }
        }
    }
    if let Some(c) = best {
        ev.prevent_default();
        app.menu_open.set(None);
        app.context_menu.set(None);
        (c.run)(app);
        true
    } else {
        false
    }
}

/// Commands for a menu, with separators between groups of different icons
/// kept simple: menus list commands in table order.
pub fn menu_items(menu: &str) -> Vec<&'static Command> {
    COMMANDS.iter().filter(|c| c.menu == menu).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tabs(c: Ctx) -> &'static [&'static str] {
        match c {
            Ctx::Any => &["start", "sch", "pcb", "3d", "device", "sym", "fp"],
            Ctx::Sch => &["sch"],
            Ctx::Pcb => &["pcb"],
            Ctx::Doc => &["sch", "pcb", "3d"],
            Ctx::Canvas => &["sch", "pcb", "sym", "fp"],
            Ctx::Sym => &["sym"],
            Ctx::Fp => &["fp"],
        }
    }

    #[test]
    fn no_two_commands_share_a_key_in_the_same_editor() {
        for (i, a) in COMMANDS.iter().enumerate() {
            for b in &COMMANDS[i + 1..] {
                if a.key.is_empty() || !a.key.eq_ignore_ascii_case(b.key) {
                    continue;
                }
                // A command for a specific editor deliberately wins over a
                // global one with the same key.
                if (a.ctx == Ctx::Any) != (b.ctx == Ctx::Any) {
                    continue;
                }
                let overlap = tabs(a.ctx).iter().any(|t| tabs(b.ctx).contains(t));
                assert!(
                    !overlap || a.run as usize == b.run as usize,
                    "{} and {} both use {}",
                    a.id,
                    b.id,
                    a.key
                );
            }
        }
    }

    #[test]
    fn ids_are_unique_and_menus_known() {
        for (i, a) in COMMANDS.iter().enumerate() {
            assert!(
                a.menu.is_empty() || MENUS.contains(&a.menu),
                "{} has unknown menu {}",
                a.id,
                a.menu
            );
            assert!(
                !COMMANDS[i + 1..].iter().any(|b| b.id == a.id),
                "duplicate id {}",
                a.id
            );
        }
    }
}
