//! Document tabs.

use crate::icons::Icon;
use crate::state::{use_app, AppState, Tab};
use leptos::prelude::*;

fn title(app: &AppState, t: &Tab) -> (String, &'static str) {
    app.project.with(|p| match t {
        Tab::Start => ("Start Page".into(), "list"),
        Tab::Schematic { board, sheet } => {
            let b = p.board(*board);
            let sheet = b.and_then(|b| b.schematic.sheets.get(*sheet)).map(|s| s.name.clone()).unwrap_or_default();
            let multi = p.boards.len() > 1;
            (if multi { format!("{sheet} · {}", b.map(|b| b.name.as_str()).unwrap_or("")) } else { sheet }, "schematic")
        }
        Tab::Pcb { board } => (format!("PCB · {}", p.board(*board).map(|b| b.name.as_str()).unwrap_or("")), "pcb"),
        Tab::View3d { board } => (format!("3D · {}", p.board(*board).map(|b| b.name.as_str()).unwrap_or("")), "cube"),
        Tab::Device { id } => (p.library.devices.get(id).map(|d| d.name.clone()).unwrap_or_else(|| id.clone()), "chip"),
        Tab::Symbol { id } => (format!("Symbol · {}", p.library.symbols.get(id).map(|d| d.name.clone()).unwrap_or_else(|| id.clone())), "schematic"),
        Tab::Footprint { id } => (format!("Footprint · {}", p.library.footprints.get(id).map(|d| d.name.clone()).unwrap_or_else(|| id.clone())), "footprint"),
    })
}

#[component]
pub fn TabStrip() -> impl IntoView {
    let app = use_app();
    view! {
        <div class="tabstrip" role="tablist">
            {move || app.tabs.get().into_iter().enumerate().map(|(i, t)| {
                let (label, icon) = title(&app, &t);
                let active = move || app.active.get() == i;
                view! {
                    <div class="tab" class:active=active role="tab"
                        on:click=move |_| { if app.active.get_untracked() != i { app.selection.set(vec![]); app.active.set(i); } }
                        on:auxclick=move |ev: web_sys::MouseEvent| if ev.button() == 1 { app.close_tab(i) }
                        title=label.clone()>
                        <Icon name=icon />
                        <span class="tab-label">{label.clone()}</span>
                        <button class="tab-close" title="Close" on:click=move |ev| { ev.stop_propagation(); app.close_tab(i); }><Icon name="x" /></button>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}
