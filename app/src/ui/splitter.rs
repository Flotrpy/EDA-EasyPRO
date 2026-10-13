//! Drag handles between panels.

use crate::state::use_app;
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Bottom,
}

/// A thin bar that resizes the neighbouring panel. Sizes are stored in the
/// settings so the layout survives reloads.
#[component]
pub fn Splitter(edge: Edge) -> impl IntoView {
    let app = use_app();
    let dragging = RwSignal::new(None::<(f64, f64)>);
    let current = move || {
        app.settings.with_untracked(|s| match edge {
            Edge::Left => s.left_width,
            Edge::Right => s.right_width,
            Edge::Bottom => s.bottom_height,
        })
    };
    let on_down = move |ev: web_sys::PointerEvent| {
        ev.prevent_default();
        if let Some(t) = ev.target() {
            use wasm_bindgen::JsCast;
            if let Ok(el) = t.dyn_into::<web_sys::Element>() {
                let _ = el.set_pointer_capture(ev.pointer_id());
            }
        }
        let start = if edge == Edge::Bottom {
            ev.client_y()
        } else {
            ev.client_x()
        } as f64;
        dragging.set(Some((start, current())));
    };
    let on_move = move |ev: web_sys::PointerEvent| {
        let Some((start, size)) = dragging.get_untracked() else {
            return;
        };
        let pos = if edge == Edge::Bottom {
            ev.client_y()
        } else {
            ev.client_x()
        } as f64;
        let delta = pos - start;
        app.settings.update(|s| match edge {
            Edge::Left => s.left_width = (size + delta).clamp(180.0, 600.0),
            Edge::Right => s.right_width = (size - delta).clamp(200.0, 600.0),
            Edge::Bottom => s.bottom_height = (size - delta).clamp(80.0, 600.0),
        });
    };
    let on_up = move |_| {
        if dragging.get_untracked().is_some() {
            dragging.set(None);
            app.settings.with_untracked(|s| s.save());
        }
    };
    let class = match edge {
        Edge::Bottom => "splitter horizontal",
        _ => "splitter vertical",
    };
    view! {
        <div class=class class:active=move || dragging.get().is_some() role="separator"
            on:pointerdown=on_down on:pointermove=on_move on:pointerup=on_up on:pointercancel=on_up></div>
    }
}
