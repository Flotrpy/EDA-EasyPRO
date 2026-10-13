//! Right-click menu on the canvas.

use crate::commands;
use crate::icons::Icon;
use crate::state::{use_app, Tab};
use leptos::prelude::*;

#[component]
pub fn ContextMenu() -> impl IntoView {
    let app = use_app();
    move || {
        app.context_menu.get().map(|(x, y)| {
            let has_sel = !app.selection.with_untracked(|s| s.is_empty());
            let ids: Vec<&'static str> = match app.tab_untracked() {
                Tab::Schematic { .. } if has_sel => vec!["edit.rotate", "edit.mirror", "edit.copy", "edit.cut", "edit.duplicate", "edit.delete", "", "view.fit"],
                Tab::Pcb { .. } if has_sel => vec!["edit.rotate", "edit.flip", "edit.copy", "edit.cut", "edit.duplicate", "edit.delete", "", "pcb.align_left", "pcb.align_top", "pcb.distribute_x", "pcb.distribute_y", "", "view.zoom_sel"],
                Tab::Symbol { .. } | Tab::Footprint { .. } if has_sel => vec!["edit.rotate", "edit.delete", "", "view.fit"],
                Tab::Schematic { .. } => vec!["edit.paste", "tool.wire", "tool.label", "tool.part", "", "view.fit", "edit.select_all"],
                Tab::Pcb { .. } => vec!["edit.paste", "tool.track", "tool.via", "pcb.rebuild", "view.clear_highlight", "", "view.fit", "edit.select_all"],
                _ => vec!["view.fit"],
            };
            view! {
                <div class="overlay transparent" on:click=move |_| app.context_menu.set(None) on:contextmenu=move |ev: web_sys::MouseEvent| { ev.prevent_default(); app.context_menu.set(None); }>
                    <div class="dropdown context" style=format!("left: {x}px; top: {y}px") on:click=|ev| ev.stop_propagation()>
                        {ids.into_iter().map(|id| {
                            if id.is_empty() { return view! { <div class="menu-sep"></div> }.into_any(); }
                            let c = commands::find(id).expect("command");
                            let key = commands::key_for(&app, c);
                            view! {
                                <button class="item" on:click=move |_| commands::run(&app, id)>
                                    <Icon name=c.icon />
                                    <span class="label">{c.label}</span>
                                    <kbd>{key}</kbd>
                                </button>
                            }.into_any()
                        }).collect_view()}
                    </div>
                </div>
            }
        })
    }
}
