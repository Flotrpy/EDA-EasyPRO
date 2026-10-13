//! 3D board preview.

use crate::icons::Icon;
use crate::state::use_app;
use easypro_core::id::Id;
use easypro_view3d::{board_mesh, OrbitCamera, View3dOptions, Viewer};
use leptos::html;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;

struct State3d {
    viewer: Viewer,
    cam: OrbitCamera,
    drag: Option<(f64, f64, bool)>,
    pending: bool,
}

fn frame(st: Rc<RefCell<State3d>>, bg: [f32; 3]) {
    if st.borrow().pending {
        return;
    }
    st.borrow_mut().pending = true;
    request_animation_frame(move || {
        let mut s = st.borrow_mut();
        s.pending = false;
        let dpr = web_sys::window()
            .map(|w| w.device_pixel_ratio())
            .unwrap_or(1.0);
        let (w, h) = s.viewer.resize(dpr);
        s.cam.aspect = (w / h.max(1.0)) as f32;
        let cam = s.cam;
        s.viewer.draw(&cam, bg);
    });
}

#[component]
pub fn View3dPane(board: Id) -> impl IntoView {
    let app = use_app();
    let node = NodeRef::<html::Canvas>::new();
    let cell: Rc<RefCell<Option<Rc<RefCell<State3d>>>>> = Rc::new(RefCell::new(None));
    let opts = RwSignal::new(View3dOptions::default());
    let error = RwSignal::new(None::<String>);
    let bg = move || {
        if app.dark.get_untracked() {
            [0.11, 0.115, 0.13]
        } else {
            [0.86, 0.88, 0.91]
        }
    };

    {
        let cell = cell.clone();
        Effect::new(move |_| {
            app.rev.track();
            app.dark.track();
            let o = opts.get();
            let Some(canvas) = node.get() else { return };
            if cell.borrow().is_none() {
                let c: web_sys::HtmlCanvasElement = canvas.unchecked_into();
                match Viewer::new(c) {
                    Ok(viewer) => {
                        *cell.borrow_mut() = Some(Rc::new(RefCell::new(State3d {
                            viewer,
                            cam: OrbitCamera::default(),
                            drag: None,
                            pending: false,
                        })))
                    }
                    Err(e) => {
                        error.set(Some(e));
                        return;
                    }
                }
            }
            let Some(st) = cell.borrow().clone() else {
                return;
            };
            let built = app
                .project
                .with_untracked(|p| p.board(board).map(|b| board_mesh(&b.pcb, &p.library, &o)));
            if let Some((mesh, radius)) = built {
                let mut s = st.borrow_mut();
                let first =
                    s.cam.target == [0.0; 3] && s.cam.distance == OrbitCamera::default().distance;
                s.viewer.set_mesh(&mesh);
                if first {
                    s.cam.fit(radius);
                    s.cam.target = [0.0, 0.0, (mesh.max[2] + mesh.min[2]) / 4.0];
                }
            }
            frame(st, bg());
        });
    }

    let with = {
        let cell = cell.clone();
        move |f: &dyn Fn(&Rc<RefCell<State3d>>)| {
            if let Some(st) = cell.borrow().clone() {
                f(&st);
                frame(st, bg());
            }
        }
    };

    let preset = {
        let with = with.clone();
        move |name: &'static str| {
            let with = with.clone();
            move |_| with(&|st| st.borrow_mut().cam.preset(name))
        }
    };

    let on_down = {
        let with = with.clone();
        move |ev: web_sys::PointerEvent| {
            if let Some(c) = node.get() {
                let c: web_sys::HtmlCanvasElement = c.unchecked_into();
                let _ = c.set_pointer_capture(ev.pointer_id());
            }
            let pan = ev.button() != 0 || ev.shift_key();
            with(&|st| {
                st.borrow_mut().drag = Some((ev.client_x() as f64, ev.client_y() as f64, pan))
            });
        }
    };
    let on_move = {
        let with = with.clone();
        move |ev: web_sys::PointerEvent| {
            with(&|st| {
                let mut s = st.borrow_mut();
                if let Some((x, y, pan)) = s.drag {
                    let (dx, dy) = (ev.client_x() as f64 - x, ev.client_y() as f64 - y);
                    if pan {
                        let h = s.viewer.canvas().client_height() as f32;
                        s.cam.pan(dx as f32, dy as f32, h);
                    } else {
                        s.cam.orbit(dx as f32, dy as f32);
                    }
                    s.drag = Some((ev.client_x() as f64, ev.client_y() as f64, pan));
                }
            });
        }
    };
    let on_up = {
        let with = with.clone();
        move |_ev: web_sys::PointerEvent| with(&|st| st.borrow_mut().drag = None)
    };
    let on_wheel = {
        let with = with.clone();
        move |ev: web_sys::WheelEvent| {
            ev.prevent_default();
            let f = (ev.delta_y() * if ev.delta_mode() == 1 { 0.05 } else { 0.0015 }).exp() as f32;
            with(&|st| st.borrow_mut().cam.zoom(f.clamp(0.5, 2.0)));
        }
    };
    let screenshot = move |_| {
        if let Some(c) = node.get() {
            let c: web_sys::HtmlCanvasElement = c.unchecked_into();
            if let Ok(url) = c.to_data_url_with_type("image/png") {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if let Ok(a) = doc.create_element("a") {
                        let a: web_sys::HtmlAnchorElement = a.unchecked_into();
                        a.set_href(&url);
                        a.set_download("board-3d.png");
                        a.click();
                    }
                }
            }
        }
    };
    {
        let cell = cell.clone();
        let h = window_event_listener(leptos::ev::resize, move |_| {
            if let Some(st) = cell.borrow().clone() {
                frame(st, bg());
            }
        });
        on_cleanup(move || h.remove());
    }

    view! {
        <div class="editor view3d">
            <canvas
                node_ref=node
                on:pointerdown=on_down
                on:pointermove=on_move
                on:pointerup=on_up
                on:wheel=on_wheel
                on:contextmenu=|ev: web_sys::MouseEvent| ev.prevent_default()
            ></canvas>
            <div class="view3d-bar">
                <button on:click=preset("iso") title="Isometric view">"3D"</button>
                <button on:click=preset("top") title="Top view">"Top"</button>
                <button on:click=preset("bottom") title="Bottom view">"Bottom"</button>
                <button on:click=preset("front") title="Front view">"Front"</button>
                <span class="sep"></span>
                <label class="chk"><input type="checkbox" prop:checked=move || opts.get().components on:change=move |_| opts.update(|o| o.components = !o.components) />"Components"</label>
                <label class="chk"><input type="checkbox" prop:checked=move || opts.get().silk on:change=move |_| opts.update(|o| o.silk = !o.silk) />"Silkscreen"</label>
                <select on:change=move |ev| { let v = event_target_value(&ev); opts.update(|o| o.gold = v == "enig"); }>
                    <option value="enig" selected=true>"ENIG (gold)"</option>
                    <option value="hasl">"HASL (silver)"</option>
                </select>
                <span class="sep"></span>
                <button on:click=screenshot title="Save a PNG image"><Icon name="download" />"PNG"</button>
            </div>
            <div class="view3d-help">"Drag to orbit · Right-drag or Shift-drag to pan · Wheel to zoom"</div>
            {move || error.get().map(|e| view! { <div class="editor-error"><strong>"3D view unavailable"</strong><p>{e}</p></div> })}
        </div>
    }
}
