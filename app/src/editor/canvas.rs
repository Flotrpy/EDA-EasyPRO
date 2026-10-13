//! The 2D canvas shared by the schematic, PCB, symbol and footprint
//! editors: camera, grid, picking, box selection, live dragging and
//! overlays. Editor-specific behaviour lives behind [`Handler`].

use crate::state::{use_app, AppState, Tab, Tool, ViewCmd};
use easypro_core::geom::{Point, Rect};
use easypro_core::id::Id;
use easypro_core::project::Project;
use easypro_core::scene::{rgba, Item, Prim, Scene, SceneLayer};
use easypro_render::gl::{GridStyle, LayerDraw};
use easypro_render::tess::{item_colored, Mesh};
use easypro_render::{Camera, Renderer};
use leptos::html;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Sch { board: Id, sheet: usize },
    Pcb { board: Id },
    Sym(String),
    Fp(String),
}

impl Kind {
    pub fn from_tab(t: &Tab) -> Option<Kind> {
        Some(match t {
            Tab::Schematic { board, sheet } => Kind::Sch {
                board: *board,
                sheet: *sheet,
            },
            Tab::Pcb { board } => Kind::Pcb { board: *board },
            Tab::Symbol { id } => Kind::Sym(id.clone()),
            Tab::Footprint { id } => Kind::Fp(id.clone()),
            _ => return None,
        })
    }
}

/// What an editor wants drawn.
pub struct Built {
    pub scene: Scene,
    pub background: [f32; 3],
    pub grid: Option<GridStyle>,
    pub order: Vec<LayerDraw>,
    /// Area for "fit all".
    pub fit: Rect,
}

/// In-progress tool interaction.
#[derive(Clone, Debug, Default)]
pub struct ToolState {
    pub points: Vec<Point>,
    pub net: String,
    pub width: i64,
    pub alt: bool,
}

#[derive(Clone, Debug)]
pub enum Gesture {
    None,
    Pan {
        last: (f64, f64),
        moved: f64,
    },
    Drag {
        start: Point,
        before: Box<Project>,
        ids: Vec<Id>,
        delta: Point,
    },
    Box {
        a: (f64, f64),
        b: (f64, f64),
    },
    Tool,
}

/// Everything that changes the static scene: revision, theme, layer
/// visibility, highlighted net, active layer and grid style.
pub type SceneKey = (
    u64,
    bool,
    Vec<easypro_core::layer::Layer>,
    Option<String>,
    easypro_core::layer::Layer,
    bool,
);

pub struct View {
    pub renderer: Renderer,
    pub camera: Camera,
    pub dpr: f64,
    pub built: Option<Built>,
    pub scene_key: Option<SceneKey>,
    pub gesture: Gesture,
    pub tool: ToolState,
    /// Snapped cursor in world nm.
    pub cursor: Point,
    /// Unsnapped cursor in world nm.
    pub raw: Point,
    pub fitted: bool,
    pub frame_pending: bool,
}

impl View {
    /// Pick tolerance: a few pixels in world units.
    pub fn tol(&self) -> i64 {
        (5.0 / self.camera.zoom * 1e6) as i64
    }
}

/// Behaviour that differs between editors.
pub trait Handler {
    fn build(&self, app: &AppState, k: &Kind) -> Option<Built>;
    fn snap(&self, app: &AppState) -> i64;
    fn hit(&self, app: &AppState, k: &Kind, p: Point, tol: i64) -> Option<Id>;
    fn in_rect(&self, app: &AppState, k: &Kind, r: Rect, crossing: bool) -> Vec<Id>;
    fn apply_move(&self, p: &mut Project, k: &Kind, ids: &[Id], d: Point);
    /// Tool press. Return true when handled.
    fn tool_down(
        &self,
        _app: &AppState,
        _k: &Kind,
        _v: &mut View,
        _ev: &web_sys::PointerEvent,
    ) -> bool {
        false
    }
    fn tool_move(&self, _app: &AppState, _k: &Kind, _v: &mut View) {}
    fn tool_up(&self, _app: &AppState, _k: &Kind, _v: &mut View) {}
    fn tool_dbl(&self, _app: &AppState, _k: &Kind, _v: &mut View) {}
    fn tool_key(&self, _app: &AppState, _k: &Kind, _v: &mut View, _key: &str) -> bool {
        false
    }
    fn overlay(&self, _app: &AppState, _k: &Kind, _v: &View, _m: &mut Mesh) {}
    fn open_item(&self, _app: &AppState, _k: &Kind, _id: Id) {}
}

pub fn handler(k: &Kind) -> &'static dyn Handler {
    match k {
        Kind::Sch { .. } => &super::sch::SchHandler,
        Kind::Pcb { .. } => &super::pcb::PcbHandler,
        Kind::Sym(_) => &super::symbol::SymHandler,
        Kind::Fp(_) => &super::footprint::FpHandler,
    }
}

thread_local! {
    static ACTIVE: RefCell<Option<(Kind, Rc<RefCell<View>>)>> = const { RefCell::new(None) };
    /// Cameras of closed or hidden editors, restored when they reopen.
    static CAMERAS: RefCell<Vec<(Kind, Camera)>> = const { RefCell::new(Vec::new()) };
}

/// Offer a key to the active editor's tool (routing shortcuts etc.).
pub fn offer_key(app: &AppState, key: &str) -> bool {
    let active = ACTIVE.with(|a| a.borrow().clone());
    let Some((k, view)) = active else {
        return false;
    };
    if Kind::from_tab(&app.tab_untracked()).as_ref() != Some(&k) {
        return false;
    }
    let handled = handler(&k).tool_key(app, &k, &mut view.borrow_mut(), key);
    if handled {
        request_frame(*app, k, view);
    }
    handled
}

/// Cancel any in-progress tool action in the active editor.
pub fn cancel_tool() {
    ACTIVE.with(|a| {
        if let Some((_, v)) = a.borrow().as_ref() {
            let mut v = v.borrow_mut();
            v.tool = ToolState::default();
            v.gesture = Gesture::None;
        }
    });
}

pub const SEL: u32 = rgba(0x3aa0ff, 255);
pub const HOVER: u32 = rgba(0x9fd0ff, 150);
pub const CROSS: u32 = rgba(0xffb030, 220);

fn scene_key(app: &AppState) -> SceneKey {
    (
        app.rev.get_untracked(),
        app.dark.get_untracked(),
        app.hidden_layers.get_untracked(),
        app.highlight_net.get_untracked(),
        app.active_layer.get_untracked(),
        app.settings.with_untracked(|s| s.grid_lines),
    )
}

fn render(app: &AppState, k: &Kind, v: &mut View) {
    let (w, h) = v.renderer.resize(v.dpr);
    v.camera.size = (w, h);
    let key = scene_key(app);
    if v.scene_key.as_ref() != Some(&key) {
        if let Some(b) = handler(k).build(app, k) {
            v.renderer.frame.origin = b.fit.center();
            if b.fit.is_empty() {
                v.renderer.frame.origin = Point::ZERO;
            }
            v.renderer.set_scene(&b.scene);
            if !v.fitted {
                v.camera.fit(
                    if b.fit.is_empty() {
                        Rect::around(Point::ZERO, 20_000_000, 20_000_000)
                    } else {
                        b.fit
                    },
                    40.0,
                );
                v.fitted = true;
            }
            v.built = Some(b);
        }
        v.scene_key = Some(key);
    }
    let mut m = Mesh::default();
    overlay(app, k, v, &mut m);
    v.renderer.set_overlay(&m);
    if let Some(b) = &v.built {
        v.renderer
            .draw(&v.camera, b.background, b.grid, &b.order, v.dpr);
    }
}

fn overlay(app: &AppState, k: &Kind, v: &View, m: &mut Mesh) {
    let frame = v.renderer.frame;
    if let Some(b) = &v.built {
        let sel = app.selection.get_untracked();
        let cross = app.cross.get_untracked();
        let hover = app.hover.get_untracked();
        if !sel.is_empty() || !cross.is_empty() || hover.is_some() {
            for (layer, items) in &b.scene.layers {
                if matches!(layer, SceneLayer::Frame) {
                    continue;
                }
                for it in items {
                    let Some(o) = it.owner else { continue };
                    if sel.contains(&o) {
                        item_colored(m, &frame, it, SEL);
                    } else if cross.contains(&o) {
                        item_colored(m, &frame, it, CROSS);
                    } else if hover == Some(o) {
                        item_colored(m, &frame, it, HOVER);
                    }
                }
            }
        }
    }
    if let Gesture::Box { a, b } = &v.gesture {
        let pa = v.camera.to_world_nm(a.0, a.1);
        let pb = v.camera.to_world_nm(b.0, b.1);
        let r = Rect::new(pa, pb);
        let crossing = b.0 < a.0;
        let color = if crossing {
            rgba(0x40c070, 60)
        } else {
            rgba(0x3a90ff, 50)
        };
        let edge = if crossing {
            rgba(0x40c070, 220)
        } else {
            rgba(0x3a90ff, 220)
        };
        item_colored(
            m,
            &frame,
            &Item {
                prim: Prim::Poly {
                    outer: r.corners().to_vec(),
                    holes: vec![],
                },
                color,
                owner: None,
            },
            color,
        );
        item_colored(
            m,
            &frame,
            &Item {
                prim: Prim::Path {
                    pts: r.corners().to_vec(),
                    w: 0,
                    closed: true,
                },
                color: edge,
                owner: None,
            },
            edge,
        );
    }
    handler(k).overlay(app, k, v, m);
    // Cursor cross-hair at the snapped point.
    if app.tool.get_untracked() != Tool::Select {
        let c = v.cursor;
        let s = (6.0 / v.camera.zoom * 1e6) as i64;
        let col = rgba(0x808080, 220);
        item_colored(
            m,
            &frame,
            &Item {
                prim: Prim::Seg {
                    a: c - Point::new(s, 0),
                    b: c + Point::new(s, 0),
                    w: 0,
                },
                color: col,
                owner: None,
            },
            col,
        );
        item_colored(
            m,
            &frame,
            &Item {
                prim: Prim::Seg {
                    a: c - Point::new(0, s),
                    b: c + Point::new(0, s),
                    w: 0,
                },
                color: col,
                owner: None,
            },
            col,
        );
    }
}

pub fn request_frame(app: AppState, k: Kind, view: Rc<RefCell<View>>) {
    if view.borrow().frame_pending {
        return;
    }
    view.borrow_mut().frame_pending = true;
    request_animation_frame(move || {
        let mut v = view.borrow_mut();
        v.frame_pending = false;
        render(&app, &k, &mut v);
    });
}

fn local_xy(ev: &web_sys::MouseEvent, canvas: &web_sys::HtmlCanvasElement) -> (f64, f64) {
    let r = canvas.get_bounding_client_rect();
    (
        ev.client_x() as f64 - r.left(),
        ev.client_y() as f64 - r.top(),
    )
}

fn update_cursor(app: &AppState, v: &mut View, xy: (f64, f64)) {
    let raw = v.camera.to_world_nm(xy.0, xy.1);
    v.raw = raw;
    let grid = match ACTIVE.with(|a| a.borrow().as_ref().map(|(k, _)| k.clone())) {
        Some(k) => handler(&k).snap(app),
        None => 0,
    };
    v.cursor = raw.snap(grid);
    app.cursor.set(Some(v.cursor));
}

#[component]
pub fn Canvas2d(kind: Kind) -> impl IntoView {
    let app = use_app();
    let node = NodeRef::<html::Canvas>::new();
    let view_cell: Rc<RefCell<Option<Rc<RefCell<View>>>>> = Rc::new(RefCell::new(None));
    let error = RwSignal::new(None::<String>);

    {
        let view_cell = view_cell.clone();
        let kind = kind.clone();
        Effect::new(move |_| {
            let Some(canvas) = node.get() else { return };
            if view_cell.borrow().is_some() {
                return;
            }
            let canvas: web_sys::HtmlCanvasElement = canvas.unchecked_into();
            match Renderer::new(canvas) {
                Ok(renderer) => {
                    let dpr = web_sys::window()
                        .map(|w| w.device_pixel_ratio())
                        .unwrap_or(1.0);
                    let saved = CAMERAS.with(|c| {
                        c.borrow()
                            .iter()
                            .find(|(k, _)| *k == kind)
                            .map(|(_, cam)| *cam)
                    });
                    let v = Rc::new(RefCell::new(View {
                        renderer,
                        camera: saved.unwrap_or_default(),
                        dpr,
                        built: None,
                        scene_key: None,
                        gesture: Gesture::None,
                        tool: ToolState::default(),
                        cursor: Point::ZERO,
                        raw: Point::ZERO,
                        fitted: saved.is_some(),
                        frame_pending: false,
                    }));
                    ACTIVE.with(|a| *a.borrow_mut() = Some((kind.clone(), v.clone())));
                    *view_cell.borrow_mut() = Some(v.clone());
                    request_frame(app, kind.clone(), v);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    }

    // Redraw on any state that affects the picture.
    {
        let view_cell = view_cell.clone();
        let kind = kind.clone();
        Effect::new(move |_| {
            app.rev.track();
            app.dark.track();
            app.selection.track();
            app.cross.track();
            app.hover.track();
            app.hidden_layers.track();
            app.active_layer.track();
            app.highlight_net.track();
            app.settings.track();
            app.tool.track();
            app.place_rot.track();
            app.place_mirror.track();
            app.show_left.track();
            app.show_right.track();
            app.show_bottom.track();
            if let Some(v) = view_cell.borrow().clone() {
                request_frame(app, kind.clone(), v);
            }
        });
    }

    // Tool switches cancel half-finished actions.
    {
        Effect::new(move |prev: Option<Tool>| {
            let t = app.tool.get();
            if prev.is_some_and(|p| p != t) {
                cancel_tool();
            }
            t
        });
    }

    // Camera commands from menus.
    {
        let view_cell = view_cell.clone();
        let kind = kind.clone();
        Effect::new(move |_| {
            let Some(cmd) = app.view_cmd.get() else {
                return;
            };
            let Some(v) = view_cell.borrow().clone() else {
                return;
            };
            {
                let mut vb = v.borrow_mut();
                let (w, h) = vb.camera.size;
                match cmd {
                    ViewCmd::Fit => {
                        if let Some(b) = &vb.built {
                            let r = b.fit;
                            vb.camera.fit(r, 40.0);
                        }
                    }
                    ViewCmd::FitRect(r) => vb
                        .camera
                        .fit(r.inflate(r.width().max(r.height()) / 4 + 500_000), 40.0),
                    ViewCmd::ZoomIn => vb.camera.zoom_at(w / 2.0, h / 2.0, 1.4),
                    ViewCmd::ZoomOut => vb.camera.zoom_at(w / 2.0, h / 2.0, 1.0 / 1.4),
                    ViewCmd::Show(p) => {
                        vb.camera.center = (p.x as f64 / 1e6, p.y as f64 / 1e6);
                        if vb.camera.zoom < 10.0 {
                            vb.camera.zoom = 10.0;
                        }
                    }
                }
            }
            app.view_cmd.set(None);
            request_frame(app, kind.clone(), v);
        });
    }

    // Resize observer substitute: redraw on window resize.
    {
        let view_cell = view_cell.clone();
        let kind = kind.clone();
        let h = window_event_listener(leptos::ev::resize, move |_| {
            if let Some(v) = view_cell.borrow().clone() {
                request_frame(app, kind.clone(), v);
            }
        });
        on_cleanup(move || h.remove());
    }

    let with_view = {
        let view_cell = view_cell.clone();
        move |f: &dyn Fn(&Rc<RefCell<View>>, &web_sys::HtmlCanvasElement)| {
            if let (Some(v), Some(c)) = (view_cell.borrow().clone(), node.get()) {
                let c: web_sys::HtmlCanvasElement = c.unchecked_into();
                f(&v, &c);
            }
        }
    };

    let on_down = {
        let with_view = with_view.clone();
        let kind = kind.clone();
        move |ev: web_sys::PointerEvent| {
            app.menu_open.set(None);
            app.context_menu.set(None);
            let kind = kind.clone();
            with_view(&|view, canvas| {
                let _ = canvas.focus();
                let _ = canvas.set_pointer_capture(ev.pointer_id());
                let xy = local_xy(&ev, canvas);
                let h = handler(&kind);
                {
                    let mut v = view.borrow_mut();
                    update_cursor(&app, &mut v, xy);
                    if ev.button() == 1 || ev.button() == 2 {
                        v.gesture = Gesture::Pan {
                            last: xy,
                            moved: 0.0,
                        };
                        return;
                    }
                    if ev.button() != 0 {
                        return;
                    }
                    if app.tool.get_untracked() != Tool::Select {
                        if h.tool_down(&app, &kind, &mut v, &ev) {
                            v.gesture = Gesture::Tool;
                        }
                    } else {
                        let tol = v.tol();
                        let hit = h.hit(&app, &kind, v.raw, tol);
                        match hit {
                            Some(id) => {
                                let mut sel = app.selection.get_untracked();
                                if ev.shift_key() || ev.ctrl_key() {
                                    if let Some(i) = sel.iter().position(|x| *x == id) {
                                        sel.remove(i);
                                    } else {
                                        sel.push(id);
                                    }
                                    app.selection.set(sel);
                                } else {
                                    if !sel.contains(&id) {
                                        sel = vec![id];
                                        app.selection.set(sel.clone());
                                    }
                                    let before = Box::new(app.project.get_untracked());
                                    v.gesture = Gesture::Drag {
                                        start: v.cursor,
                                        before,
                                        ids: sel,
                                        delta: Point::ZERO,
                                    };
                                }
                            }
                            None => {
                                if !ev.shift_key() {
                                    app.selection.set(vec![]);
                                }
                                v.gesture = Gesture::Box { a: xy, b: xy };
                            }
                        }
                    }
                }
                request_frame(app, kind.clone(), view.clone());
            });
        }
    };

    let on_move = {
        let with_view = with_view.clone();
        let kind = kind.clone();
        move |ev: web_sys::PointerEvent| {
            let kind = kind.clone();
            with_view(&|view, canvas| {
                let xy = local_xy(&ev, canvas);
                let h = handler(&kind);
                {
                    let mut v = view.borrow_mut();
                    update_cursor(&app, &mut v, xy);
                    let mut g = std::mem::replace(&mut v.gesture, Gesture::None);
                    match &mut g {
                        Gesture::Pan { last, moved } => {
                            v.camera.pan_pixels(xy.0 - last.0, xy.1 - last.1);
                            *moved += (xy.0 - last.0).abs() + (xy.1 - last.1).abs();
                            *last = xy;
                        }
                        Gesture::Drag {
                            start,
                            before,
                            ids,
                            delta,
                        } => {
                            let d = v.cursor - *start;
                            if d != *delta {
                                *delta = d;
                                let mut p = (**before).clone();
                                h.apply_move(&mut p, &kind, ids, d);
                                app.project.set(p);
                                app.rev.update(|r| *r += 1);
                            }
                        }
                        Gesture::Box { b, .. } => *b = xy,
                        Gesture::Tool | Gesture::None => {
                            if app.tool.get_untracked() == Tool::Select {
                                let tol = v.tol();
                                let hv = h.hit(&app, &kind, v.raw, tol);
                                if app.hover.get_untracked() != hv {
                                    app.hover.set(hv);
                                }
                            } else {
                                h.tool_move(&app, &kind, &mut v);
                            }
                        }
                    }
                    v.gesture = g;
                }
                request_frame(app, kind.clone(), view.clone());
            });
        }
    };

    let on_up = {
        let with_view = with_view.clone();
        let kind = kind.clone();
        move |ev: web_sys::PointerEvent| {
            let kind = kind.clone();
            with_view(&|view, canvas| {
                let _ = canvas.release_pointer_capture(ev.pointer_id());
                let xy = local_xy(&ev, canvas);
                let h = handler(&kind);
                {
                    let mut v = view.borrow_mut();
                    let g = std::mem::replace(&mut v.gesture, Gesture::None);
                    match g {
                        Gesture::Pan { moved, .. } => {
                            if ev.button() == 2 && moved < 4.0 {
                                if matches!(app.tool.get_untracked(), Tool::Select) {
                                    // Right click: select what is under the cursor, then menu.
                                    let tol = v.tol();
                                    if let Some(id) = h.hit(&app, &kind, v.raw, tol) {
                                        if !app.selection.get_untracked().contains(&id) {
                                            app.selection.set(vec![id]);
                                        }
                                    }
                                    app.context_menu
                                        .set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                                } else {
                                    // Right click ends the current tool action.
                                    h.tool_dbl(&app, &kind, &mut v);
                                    v.tool = ToolState::default();
                                }
                            }
                        }
                        Gesture::Drag {
                            before, delta, ids, ..
                        } => {
                            if delta != Point::ZERO {
                                app.history.update_value(|hist| hist.push("Move", *before));
                                app.touch();
                                app.status.set(format!("Moved {} object(s)", ids.len()));
                            }
                        }
                        Gesture::Box { a, .. } => {
                            let pa = v.camera.to_world_nm(a.0, a.1);
                            let pb = v.camera.to_world_nm(xy.0, xy.1);
                            if (a.0 - xy.0).abs() + (a.1 - xy.1).abs() > 3.0 {
                                let crossing = xy.0 < a.0;
                                let found = h.in_rect(&app, &kind, Rect::new(pa, pb), crossing);
                                let mut sel = if ev.shift_key() {
                                    app.selection.get_untracked()
                                } else {
                                    vec![]
                                };
                                for f in found {
                                    if !sel.contains(&f) {
                                        sel.push(f);
                                    }
                                }
                                app.selection.set(sel);
                            }
                        }
                        Gesture::Tool => h.tool_up(&app, &kind, &mut v),
                        Gesture::None => {}
                    }
                }
                request_frame(app, kind.clone(), view.clone());
            });
        }
    };

    let on_dbl = {
        let with_view = with_view.clone();
        let kind = kind.clone();
        move |_ev: web_sys::MouseEvent| {
            let kind = kind.clone();
            with_view(&|view, _canvas| {
                let h = handler(&kind);
                {
                    let mut v = view.borrow_mut();
                    if app.tool.get_untracked() == Tool::Select {
                        let tol = v.tol();
                        if let Some(id) = h.hit(&app, &kind, v.raw, tol) {
                            h.open_item(&app, &kind, id);
                        }
                    } else {
                        h.tool_dbl(&app, &kind, &mut v);
                    }
                }
                request_frame(app, kind.clone(), view.clone());
            });
        }
    };

    let on_wheel = {
        let with_view = with_view.clone();
        let kind = kind.clone();
        move |ev: web_sys::WheelEvent| {
            ev.prevent_default();
            let kind = kind.clone();
            with_view(&|view, canvas| {
                let xy = local_xy(&ev, canvas);
                {
                    let mut v = view.borrow_mut();
                    if ev.shift_key() {
                        v.camera.pan_pixels(-ev.delta_y(), 0.0);
                    } else if ev.ctrl_key()
                        || ev.delta_mode() != 0
                        || ev.delta_y().abs() >= 4.0
                        || ev.delta_x() == 0.0
                    {
                        let f = (-ev.delta_y() * if ev.delta_mode() == 1 { 0.05 } else { 0.0015 })
                            .exp();
                        v.camera.zoom_at(xy.0, xy.1, f.clamp(0.5, 2.0));
                    } else {
                        v.camera.pan_pixels(-ev.delta_x(), -ev.delta_y());
                    }
                    update_cursor(&app, &mut v, xy);
                }
                request_frame(app, kind.clone(), view.clone());
            });
        }
    };

    let on_leave = move |_| {
        app.cursor.set(None);
        app.hover.set(None);
    };

    {
        let kind = kind.clone();
        let view_cell = send_wrapper::SendWrapper::new(view_cell.clone());
        on_cleanup(move || {
            if let Some(v) = view_cell.borrow().as_ref() {
                let cam = v.borrow().camera;
                CAMERAS.with(|c| {
                    let mut c = c.borrow_mut();
                    c.retain(|(k, _)| *k != kind);
                    c.push((kind.clone(), cam));
                });
            }
            ACTIVE.with(|a| {
                let mut a = a.borrow_mut();
                if a.as_ref().is_some_and(|(k, _)| *k == kind) {
                    *a = None;
                }
            });
        });
    }

    view! {
        <div class="editor">
            <canvas
                node_ref=node
                tabindex="0"
                on:pointerdown=on_down
                on:pointermove=on_move
                on:pointerup=on_up
                on:dblclick=on_dbl
                on:wheel=on_wheel
                on:pointerleave=on_leave
                on:contextmenu=|ev: web_sys::MouseEvent| ev.prevent_default()
            ></canvas>
            {move || error.get().map(|e| view! {
                <div class="editor-error">
                    <strong>"Cannot start the drawing canvas"</strong>
                    <p>{e}</p>
                    <p>"EasyPRO needs WebGL2. Enable hardware acceleration or try a current Chrome, Edge or Firefox."</p>
                </div>
            })}
        </div>
    }
}

/// Draw items into the overlay with a colour.
pub fn draw_items(v: &View, m: &mut Mesh, items: &[Item]) {
    let frame = v.renderer.frame;
    for it in items {
        item_colored(m, &frame, it, it.color);
    }
}
