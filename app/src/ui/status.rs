//! Status bar: messages, cursor position, grid and units.

use crate::state::{use_app, Tab};
use leptos::prelude::*;

#[component]
pub fn StatusBar() -> impl IntoView {
    let app = use_app();
    let pos = move || {
        let u = app.settings.with(|s| s.unit);
        app.cursor.get().map(|p| format!("X {}  Y {} {}", u.format(p.x), u.format(p.y), u.label())).unwrap_or_default()
    };
    let grid = move || {
        let u = app.settings.with(|s| s.unit);
        let g = match app.tab() {
            Tab::Pcb { .. } | Tab::Footprint { .. } => app.settings.with(|s| s.pcb_grid),
            Tab::Schematic { .. } | Tab::Symbol { .. } => app.settings.with(|s| s.sch_grid),
            _ => return String::new(),
        };
        format!("Grid {} {}", u.format(g), u.label())
    };
    view! {
        <footer class="statusbar">
            <span class="msg">{move || app.status.get()}</span>
            <span class="spacer"></span>
            <span class="sel">{move || { let n = app.selection.with(|s| s.len()); if n > 0 { format!("{n} selected") } else { String::new() } }}</span>
            <span class="pos">{pos}</span>
            <span class="grid">{grid}</span>
            <button class="unit" title="Toggle units" on:click=move |_| crate::actions::toggle_units(&app)>{move || app.settings.with(|s| s.unit.label())}</button>
            <span class="saved">{move || if app.dirty.get() { "Unsaved · kept in browser" } else { "Saved" }}</span>
        </footer>
    }
}
