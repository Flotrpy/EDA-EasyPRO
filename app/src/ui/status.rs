//! Status bar: messages, cursor position, grid and units.

use crate::state::{use_app, Tab};
use leptos::prelude::*;

#[component]
pub fn StatusBar() -> impl IntoView {
    let app = use_app();
    let pos = move || {
        let u = app.settings.with(|s| s.unit);
        app.cursor
            .get()
            .map(|p| format!("X {}  Y {} {}", u.format(p.x), u.format(p.y), u.label()))
            .unwrap_or_default()
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
            <span class="hover-info">{move || app.hover_info.get()}</span>
            <span class="spacer"></span>
            <span class="sel">{move || { let n = app.selection.with(|s| s.len()); if n > 0 { format!("{n} selected") } else { String::new() } }}</span>
            {move || (app.pcb_flipped.get() && matches!(app.tab(), Tab::Pcb { .. })).then(|| view! { <span class="flipped">"Bottom view"</span> })}
            <span class="pos">{pos}</span>
            <button class="grid link" title="Click to change the grid" on:click=move |_| cycle_grid(&app)>{grid}</button>
            <button class="unit" title="Toggle units" on:click=move |_| crate::actions::toggle_units(&app)>{move || app.settings.with(|s| s.unit.label())}</button>
            <span class="saved">{move || if app.dirty.get() { "Unsaved · kept in browser" } else { "Saved" }}</span>
        </footer>
    }
}

/// Step through common grid sizes for the active editor.
fn cycle_grid(app: &crate::state::AppState) {
    use easypro_core::units::{mil, mm};
    let pcb = matches!(app.tab_untracked(), Tab::Pcb { .. } | Tab::Footprint { .. });
    app.settings.update(|s| {
        if pcb {
            let steps = [
                mm(0.05),
                mm(0.1),
                mm(0.25),
                mm(0.5),
                mm(1.0),
                mil(5.0),
                mil(10.0),
                mil(25.0),
                mil(50.0),
            ];
            let i = steps
                .iter()
                .position(|g| *g == s.pcb_grid)
                .map(|i| (i + 1) % steps.len())
                .unwrap_or(2);
            s.pcb_grid = steps[i];
        } else {
            let steps = [mil(10.0), mil(25.0), mil(50.0), mil(100.0)];
            let i = steps
                .iter()
                .position(|g| *g == s.sch_grid)
                .map(|i| (i + 1) % steps.len())
                .unwrap_or(2);
            s.sch_grid = steps[i];
        }
    });
    app.settings.with_untracked(|s| s.save());
}
