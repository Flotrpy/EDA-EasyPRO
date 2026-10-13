//! Application state shared through Leptos context.

use crate::storage;
use easypro_core::check::Issue;
use easypro_core::geom::Point;
use easypro_core::history::History;
use easypro_core::id::Id;
use easypro_core::layer::Layer;
use easypro_core::project::Project;
use easypro_core::units::{mil, mm, Unit};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    pub unit: Unit,
    pub sch_grid: i64,
    pub pcb_grid: i64,
    pub grid_lines: bool,
    pub autosave: bool,
    pub realtime_drc: bool,
    pub snap_to_pins: bool,
    /// Command id -> key chord overrides.
    pub hotkeys: BTreeMap<String, String>,
    pub fab: String,
    pub left_width: f64,
    pub right_width: f64,
    pub bottom_height: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::System,
            unit: Unit::Mm,
            sch_grid: mil(50.0),
            pcb_grid: mm(0.25),
            grid_lines: false,
            autosave: true,
            realtime_drc: true,
            snap_to_pins: true,
            hotkeys: BTreeMap::new(),
            fab: "jlcpcb".into(),
            left_width: 260.0,
            right_width: 280.0,
            bottom_height: 170.0,
        }
    }
}

const SETTINGS_KEY: &str = "easypro.settings";

impl Settings {
    pub fn load() -> Settings {
        storage::get_pref(SETTINGS_KEY)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        storage::set_pref(SETTINGS_KEY, &serde_json::to_string(self).unwrap());
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tab {
    Start,
    Schematic { board: Id, sheet: usize },
    Pcb { board: Id },
    View3d { board: Id },
    Device { id: String },
    Symbol { id: String },
    Footprint { id: String },
}

impl Tab {
    pub fn board(&self) -> Option<Id> {
        match self {
            Tab::Schematic { board, .. } | Tab::Pcb { board } | Tab::View3d { board } => {
                Some(*board)
            }
            _ => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Tab::Start => "start",
            Tab::Schematic { .. } => "sch",
            Tab::Pcb { .. } => "pcb",
            Tab::View3d { .. } => "3d",
            Tab::Device { .. } => "device",
            Tab::Symbol { .. } => "symbol",
            Tab::Footprint { .. } => "footprint",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Select,
    // Schematic
    Wire,
    Label,
    Power,
    Ground,
    NoConnect,
    Text,
    Place(String),
    // PCB
    Track,
    Via,
    Pour,
    Outline,
    Line,
    Measure,
    // Symbol editor
    Pin,
    SymLine,
    SymRect,
    SymCircle,
    // Footprint editor
    PadSmd,
    PadTht,
    FpLine,
    FpRect,
    FpCircle,
}

impl Tool {
    pub fn hint(&self) -> &'static str {
        match self {
            Tool::Select => "Click to select, drag to move, drag on empty space to box-select. Right-drag or middle-drag pans.",
            Tool::Wire => "Click to start a wire, click to add corners, double-click or Esc to finish. / changes the bend.",
            Tool::Label => "Click on a wire or pin to attach a net label.",
            Tool::Power => "Click to place a power flag (rename it in Properties).",
            Tool::Ground => "Click to place a ground flag.",
            Tool::NoConnect => "Click a pin to mark it intentionally unconnected.",
            Tool::Text => "Click to place a text note.",
            Tool::Place(_) => "Click to place. Space or R rotates, X mirrors, Esc stops placing.",
            Tool::Track => "Click a pad to start routing. V places a via and changes layer, / changes the bend, Esc finishes.",
            Tool::Via => "Click to place a via.",
            Tool::Pour => "Click the corners of the copper area, double-click to close.",
            Tool::Outline => "Drag a rectangle to set the board outline.",
            Tool::Line => "Click to draw silkscreen lines on the active layer; double-click to finish.",
            Tool::Measure => "Click two points to measure.",
            Tool::Pin => "Click to add a pin. Space rotates it.",
            Tool::SymLine | Tool::FpLine => "Click to draw lines; double-click to finish.",
            Tool::SymRect | Tool::FpRect => "Drag to draw a rectangle.",
            Tool::SymCircle | Tool::FpCircle => "Drag from the centre to draw a circle.",
            Tool::PadSmd => "Click to add an SMD pad; numbers increase automatically.",
            Tool::PadTht => "Click to add a through-hole pad.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Ok,
    Warn,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogLine {
    pub level: LogLevel,
    pub text: String,
    pub n: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftTab {
    Projects,
    Library,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BottomTab {
    Log,
    Issues,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Dialog {
    Settings,
    Shortcuts,
    About,
    Fabrication,
    Rules,
    BoardSetup,
    History,
    NewProject,
    Rename,
    IcWizard,
}

/// Requests from menus to the active editor's camera.
#[derive(Clone, Debug, PartialEq)]
pub enum ViewCmd {
    Fit,
    /// Fit a world rectangle (zoom to selection).
    FitRect(easypro_core::geom::Rect),
    ZoomIn,
    ZoomOut,
    Show(Point),
}

#[derive(Clone, Debug)]
pub enum Clip {
    Sch(easypro_core::edit::SchFragment),
    Pcb(easypro_core::edit::PcbFragment),
}

#[derive(Clone, Copy)]
pub struct AppState {
    pub project: RwSignal<Project>,
    pub history: StoredValue<History<Project>>,
    /// Bumped on every document change (drives scene rebuilds).
    pub rev: RwSignal<u64>,
    pub key: RwSignal<String>,
    pub dirty: RwSignal<bool>,
    pub saved_name: RwSignal<Option<String>>,
    pub tabs: RwSignal<Vec<Tab>>,
    pub active: RwSignal<usize>,
    pub selection: RwSignal<Vec<Id>>,
    /// Objects linked to the selection in the other editor (cross-probing).
    pub cross: RwSignal<Vec<Id>>,
    /// Id under the cursor in the active editor.
    pub hover: RwSignal<Option<Id>>,
    /// Short description of the object under the cursor.
    pub hover_info: RwSignal<String>,
    pub tool: RwSignal<Tool>,
    pub settings: RwSignal<Settings>,
    pub log: RwSignal<Vec<LogLine>>,
    pub issues: RwSignal<Vec<Issue>>,
    pub issues_title: RwSignal<String>,
    pub show_left: RwSignal<bool>,
    pub show_right: RwSignal<bool>,
    pub show_bottom: RwSignal<bool>,
    pub left_tab: RwSignal<LeftTab>,
    pub bottom_tab: RwSignal<BottomTab>,
    pub dialog: RwSignal<Option<Dialog>>,
    pub palette: RwSignal<bool>,
    pub cursor: RwSignal<Option<Point>>,
    pub status: RwSignal<String>,
    pub active_layer: RwSignal<Layer>,
    pub hidden_layers: RwSignal<Vec<Layer>>,
    pub highlight_net: RwSignal<Option<String>>,
    pub view_cmd: RwSignal<Option<ViewCmd>>,
    pub menu_open: RwSignal<Option<&'static str>>,
    pub context_menu: RwSignal<Option<(f64, f64)>>,
    pub clipboard: StoredValue<Option<Clip>>,
    pub recent: RwSignal<Vec<storage::ProjectMeta>>,
    pub library_query: RwSignal<String>,
    pub library_pick: RwSignal<Option<String>>,
    /// Rotation and mirroring applied to the part or pin being placed.
    pub place_rot: RwSignal<f64>,
    pub place_mirror: RwSignal<bool>,
    /// Track width chosen in the toolbar; None follows the design rules.
    pub route_width: RwSignal<Option<i64>>,
    /// Recently placed device ids.
    pub recent_parts: RwSignal<Vec<String>>,
    pub dark: Memo<bool>,
}

impl AppState {
    pub fn new() -> AppState {
        let settings = RwSignal::new(Settings::load());
        let prefers_dark = web_sys::window()
            .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
            .map(|m| m.matches())
            .unwrap_or(false);
        let dark = Memo::new(move |_| match settings.with(|s| s.theme) {
            Theme::Dark => true,
            Theme::Light => false,
            Theme::System => prefers_dark,
        });
        AppState {
            project: RwSignal::new(Project::new("Untitled")),
            history: StoredValue::new(History::new(200)),
            rev: RwSignal::new(0),
            key: RwSignal::new(storage::new_key()),
            dirty: RwSignal::new(false),
            saved_name: RwSignal::new(None),
            tabs: RwSignal::new(vec![Tab::Start]),
            active: RwSignal::new(0),
            selection: RwSignal::new(vec![]),
            cross: RwSignal::new(vec![]),
            hover: RwSignal::new(None),
            hover_info: RwSignal::new(String::new()),
            tool: RwSignal::new(Tool::Select),
            settings,
            log: RwSignal::new(vec![]),
            issues: RwSignal::new(vec![]),
            issues_title: RwSignal::new(String::new()),
            show_left: RwSignal::new(true),
            show_right: RwSignal::new(true),
            show_bottom: RwSignal::new(true),
            left_tab: RwSignal::new(LeftTab::Projects),
            bottom_tab: RwSignal::new(BottomTab::Log),
            dialog: RwSignal::new(None),
            palette: RwSignal::new(false),
            cursor: RwSignal::new(None),
            status: RwSignal::new(String::new()),
            active_layer: RwSignal::new(Layer::TopCopper),
            hidden_layers: RwSignal::new(vec![
                Layer::TopAssembly,
                Layer::BottomAssembly,
                Layer::Document,
            ]),
            highlight_net: RwSignal::new(None),
            view_cmd: RwSignal::new(None),
            menu_open: RwSignal::new(None),
            context_menu: RwSignal::new(None),
            clipboard: StoredValue::new(None),
            recent: RwSignal::new(vec![]),
            library_query: RwSignal::new(String::new()),
            library_pick: RwSignal::new(None),
            place_rot: RwSignal::new(0.0),
            place_mirror: RwSignal::new(false),
            route_width: RwSignal::new(None),
            recent_parts: RwSignal::new(storage::recent_parts()),
            dark,
        }
    }

    pub fn tab(&self) -> Tab {
        let i = self.active.get();
        self.tabs.with(|t| t.get(i).cloned().unwrap_or(Tab::Start))
    }

    pub fn tab_untracked(&self) -> Tab {
        let i = self.active.get_untracked();
        self.tabs
            .with_untracked(|t| t.get(i).cloned().unwrap_or(Tab::Start))
    }

    /// Open (or focus) a tab.
    pub fn open(&self, tab: Tab) {
        let existing = self
            .tabs
            .with_untracked(|t| t.iter().position(|x| *x == tab));
        let idx = match existing {
            Some(i) => i,
            None => {
                self.tabs.update(|t| t.push(tab));
                self.tabs.with_untracked(|t| t.len() - 1)
            }
        };
        if idx != self.active.get_untracked() {
            self.selection.set(vec![]);
            self.tool.set(Tool::Select);
            self.active.set(idx);
        }
    }

    pub fn close_tab(&self, idx: usize) {
        let len = self.tabs.with_untracked(|t| t.len());
        if idx >= len {
            return;
        }
        self.tabs.update(|t| {
            t.remove(idx);
            if t.is_empty() {
                t.push(Tab::Start);
            }
        });
        let a = self.active.get_untracked();
        let new_len = self.tabs.with_untracked(|t| t.len());
        if a >= idx && a > 0 {
            self.active.set((a - 1).min(new_len - 1));
        } else {
            self.active.set(a.min(new_len - 1));
        }
        self.selection.set(vec![]);
        self.tool.set(Tool::Select);
    }

    /// Apply an undoable change to the project.
    pub fn edit(&self, label: &str, f: impl FnOnce(&mut Project)) {
        let before = self.project.get_untracked();
        self.project.update(f);
        self.history.update_value(|h| h.push(label, before));
        self.touch();
    }

    /// Like [`AppState::edit`] but the closure decides whether anything
    /// changed; nothing is recorded when it returns false.
    pub fn try_edit(&self, label: &str, f: impl FnOnce(&mut Project) -> bool) -> bool {
        let before = self.project.get_untracked();
        let mut changed = false;
        self.project.update(|p| changed = f(p));
        if changed {
            self.history.update_value(|h| h.push(label, before));
            self.touch();
        } else {
            self.project.set(before);
        }
        changed
    }

    pub fn touch(&self) {
        self.dirty.set(true);
        self.rev.update(|r| *r += 1);
    }

    pub fn undo(&self) {
        let mut label = None;
        self.project
            .update(|p| self.history.update_value(|h| label = h.undo(p)));
        match label {
            Some(l) => {
                self.selection.set(vec![]);
                self.touch();
                self.info(format!("Undo: {l}"));
            }
            None => self.status.set("Nothing to undo".into()),
        }
    }

    pub fn redo(&self) {
        let mut label = None;
        self.project
            .update(|p| self.history.update_value(|h| label = h.redo(p)));
        match label {
            Some(l) => {
                self.selection.set(vec![]);
                self.touch();
                self.info(format!("Redo: {l}"));
            }
            None => self.status.set("Nothing to redo".into()),
        }
    }

    fn push_log(&self, level: LogLevel, text: String) {
        self.status.set(text.clone());
        self.log.update(|l| {
            let n = l.last().map(|x| x.n + 1).unwrap_or(0);
            l.push(LogLine { level, text, n });
            if l.len() > 500 {
                l.remove(0);
            }
        });
    }

    pub fn info(&self, t: impl Into<String>) {
        self.push_log(LogLevel::Info, t.into());
    }

    pub fn ok(&self, t: impl Into<String>) {
        self.push_log(LogLevel::Ok, t.into());
    }

    pub fn warn(&self, t: impl Into<String>) {
        self.push_log(LogLevel::Warn, t.into());
    }

    pub fn error(&self, t: impl Into<String>) {
        self.push_log(LogLevel::Error, t.into());
        self.show_bottom.set(true);
        self.bottom_tab.set(BottomTab::Log);
    }

    /// Replace the open project (open file, new project, example...).
    pub fn load_project(&self, p: Project, key: String) {
        self.project.set(p);
        self.key.set(key);
        self.history.update_value(|h| h.clear());
        self.selection.set(vec![]);
        self.issues.set(vec![]);
        self.highlight_net.set(None);
        self.dirty.set(false);
        self.saved_name.set(None);
        let boards = self
            .project
            .with_untracked(|p| p.boards.iter().map(|b| b.id).collect::<Vec<_>>());
        let mut tabs = vec![Tab::Start];
        if let Some(b) = boards.first() {
            tabs.push(Tab::Schematic {
                board: *b,
                sheet: 0,
            });
            tabs.push(Tab::Pcb { board: *b });
        }
        self.tabs.set(tabs);
        self.active.set(if boards.is_empty() { 0 } else { 1 });
        self.tool.set(Tool::Select);
        self.rev.update(|r| *r += 1);
    }

    pub fn unit(&self) -> Unit {
        self.settings.with_untracked(|s| s.unit)
    }

    pub fn fmt_len(&self, nm: i64) -> String {
        let u = self.settings.with(|s| s.unit);
        format!("{} {}", u.format(nm), u.label())
    }
}

pub fn use_app() -> AppState {
    use_context::<AppState>().expect("app state")
}
