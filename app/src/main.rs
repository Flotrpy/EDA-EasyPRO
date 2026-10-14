//! EasyPRO web application.

mod actions;
mod commands;
mod editor;
mod icons;
mod js;
mod state;
mod storage;
mod ui;

use leptos::prelude::*;
use leptos::task::spawn_local;
use state::{use_app, AppState, Tab};
use wasm_bindgen::JsCast;

const LAST_KEY: &str = "easypro.last";

fn typing_target(ev: &web_sys::KeyboardEvent) -> bool {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .map(|e| {
            matches!(e.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                || e.has_attribute("contenteditable")
        })
        .unwrap_or(false)
}

fn install_keyboard(app: AppState) {
    let h = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.default_prevented() {
            return;
        }
        let ch = commands::chord(&ev);
        if typing_target(&ev)
            && !matches!(
                ch.as_str(),
                "Escape" | "Ctrl+S" | "Ctrl+K" | "Ctrl+Z" | "Ctrl+Y"
            )
        {
            return;
        }
        if typing_target(&ev) && matches!(ch.as_str(), "Ctrl+Z" | "Ctrl+Y") {
            return;
        }
        if app.palette.get_untracked() || app.dialog.get_untracked().is_some() {
            if ch == "Escape" {
                ev.prevent_default();
                actions::escape(&app);
            }
            return;
        }
        let key = if ev.key().len() == 1 {
            ev.key().to_uppercase()
        } else {
            ev.key()
        };
        if !ev.ctrl_key()
            && !ev.meta_key()
            && !ev.alt_key()
            && editor::canvas::offer_key(&app, &key)
        {
            ev.prevent_default();
            return;
        }
        commands::handle_key(&app, &ev);
    });
    std::mem::forget(h);
}

/// Drop project files or KiCad libraries anywhere on the window.
fn install_file_drop(app: AppState) {
    let over = window_event_listener(leptos::ev::dragover, |ev| ev.prevent_default());
    let drop = window_event_listener(leptos::ev::drop, move |ev| {
        ev.prevent_default();
        let Some(file) = ev
            .data_transfer()
            .and_then(|d| d.files())
            .and_then(|f| f.get(0))
        else {
            return;
        };
        let name = file.name();
        spawn_local(async move {
            match wasm_bindgen_futures::JsFuture::from(file.text())
                .await
                .ok()
                .and_then(|v| v.as_string())
            {
                Some(text) => actions::open_dropped(&app, &name, &text),
                None => app.error(format!("Could not read {name}")),
            }
        });
    });
    std::mem::forget((over, drop));
}

/// With autosave off, ask before leaving the page with unsaved edits.
fn install_unload_guard(app: AppState) {
    let h = window_event_listener(leptos::ev::beforeunload, move |ev| {
        if app.dirty.get_untracked() && !app.settings.with_untracked(|s| s.autosave) {
            ev.prevent_default();
            ev.set_return_value("You have unsaved changes.");
        }
    });
    std::mem::forget(h);
}

fn install_autosave(app: AppState) {
    let pending = StoredValue::new(0u64);
    Effect::new(move |_| {
        let rev = app.rev.get();
        if !app.dirty.get_untracked() || !app.settings.with_untracked(|s| s.autosave) {
            return;
        }
        pending.set_value(rev);
        set_timeout(
            move || {
                if pending.get_value() != rev {
                    return;
                }
                let key = app.key.get_untracked();
                app.project.update_untracked(|p| p.modified = js::now_iso());
                let p = app.project.get_untracked();
                spawn_local(async move {
                    if storage::save_local(&key, &p).await.is_ok() {
                        storage::set_pref(LAST_KEY, &key);
                        app.recent.set(storage::list_local().await);
                    }
                });
            },
            std::time::Duration::from_millis(800),
        );
    });
}

/// Re-run DRC quietly after PCB edits so problems show up as you work.
fn install_live_drc(app: AppState) {
    let pending = StoredValue::new(0u64);
    Effect::new(move |_| {
        let rev = app.rev.get();
        let tab = app.tab();
        let Tab::Pcb { board } = tab else { return };
        if !app.settings.with_untracked(|s| s.realtime_drc) {
            return;
        }
        pending.set_value(rev);
        set_timeout(
            move || {
                if pending.get_value() != rev {
                    return;
                }
                let unit = app.unit();
                let issues = app.project.with_untracked(|p| {
                    p.board(board)
                        .map(|b| easypro_core::drc::run(&b.pcb, &p.library, unit))
                        .unwrap_or_default()
                });
                // Unrouted nets are expected while laying out; keep live markers to real errors.
                let errors: Vec<_> = issues
                    .into_iter()
                    .filter(|i| i.severity == easypro_core::check::Severity::Error)
                    .collect();
                let title = format!("Live DRC: {} error(s)", errors.len());
                if app.issues_title.get_untracked().starts_with("Live DRC")
                    || app.issues.with_untracked(|i| i.is_empty())
                {
                    app.issues_title.set(title);
                    app.issues.set(errors);
                }
            },
            std::time::Duration::from_millis(600),
        );
    });
}

/// Selecting parts in one editor highlights the linked parts in the other.
fn install_cross_probe(app: AppState) {
    Effect::new(move |_| {
        let sel = app.selection.get();
        let tab = app.tab_untracked();
        let Some(b) = tab.board() else { return };
        let cross = app.project.with_untracked(|p| {
            let Some(bd) = p.board(b) else { return vec![] };
            match tab {
                Tab::Schematic { .. } => bd
                    .pcb
                    .components
                    .iter()
                    .filter(|c| c.link.is_some_and(|l| sel.contains(&l)))
                    .map(|c| c.id)
                    .collect(),
                Tab::Pcb { .. } => bd
                    .pcb
                    .components
                    .iter()
                    .filter(|c| sel.contains(&c.id))
                    .filter_map(|c| c.link)
                    .collect(),
                _ => vec![],
            }
        });
        // Keep the other editor's view of the selection for when the user switches.
        if !cross.is_empty() {
            app.cross.set(cross);
        }
    });
    // On switching editors, show the cross-probed parts as highlighted.
    Effect::new(move |prev: Option<String>| {
        let k = app.tab().kind().to_string();
        if prev.as_deref().is_some_and(|p| p != k) && !matches!(k.as_str(), "sch" | "pcb") {
            app.cross.set(vec![]);
        }
        k
    });
}

/// Remember which panels are open.
fn install_panel_memory(app: AppState) {
    Effect::new(move |_| {
        let panels = (
            app.show_left.get(),
            app.show_right.get(),
            app.show_bottom.get(),
        );
        if app.settings.with_untracked(|s| s.panels) != panels {
            app.settings.update_untracked(|s| s.panels = panels);
            app.settings.with_untracked(|s| s.save());
        }
    });
}

fn install_theme(app: AppState) {
    Effect::new(move |_| {
        let dark = app.dark.get();
        if let Some(root) = document().document_element() {
            let _ = root.set_attribute("data-theme", if dark { "dark" } else { "light" });
        }
    });
}

fn restore_last(app: AppState) {
    spawn_local(async move {
        app.recent.set(storage::list_local().await);
        if let Some(key) = storage::get_pref(LAST_KEY) {
            if let Ok(p) = storage::load_local(&key).await {
                let name = p.name.clone();
                app.load_project(p, key);
                app.info(format!("Restored \"{name}\" from this browser"));
                return;
            }
        }
        app.info("Welcome to EasyPRO. Start with File → New Project or open the example.");
    });
    Effect::new(move |_| {
        let key = app.key.get();
        storage::set_pref(LAST_KEY, &key);
    });
}

#[component]
fn Shell() -> impl IntoView {
    let app = use_app();
    view! {
        <div class="app" on:click=move |_| app.menu_open.set(None)>
            <ui::topbar::TopBar />
            <ui::toolbar::Toolbar />
            <div class="main">
                <Show when=move || app.show_left.get()><ui::left::LeftPanel /><ui::splitter::Splitter edge=ui::splitter::Edge::Left /></Show>
                <div class="center">
                    <ui::tabs::TabStrip />
                    <div class="doc"><editor::Workspace /></div>
                    <Show when=move || app.show_bottom.get()><ui::splitter::Splitter edge=ui::splitter::Edge::Bottom /><ui::bottom::BottomPanel /></Show>
                </div>
                <Show when=move || app.show_right.get() && !matches!(app.tab(), Tab::Start)><ui::splitter::Splitter edge=ui::splitter::Edge::Right /><ui::right::RightPanel /></Show>
            </div>
            <ui::status::StatusBar />
            <ui::context::ContextMenu />
            <ui::dialogs::Dialogs />
            <Show when=move || app.palette.get()><ui::palette::Palette /></Show>
        </div>
    }
}

#[component]
fn App() -> impl IntoView {
    let app = AppState::new();
    provide_context(app);
    install_keyboard(app);
    install_autosave(app);
    install_file_drop(app);
    install_unload_guard(app);
    install_live_drc(app);
    install_cross_probe(app);
    install_theme(app);
    install_panel_memory(app);
    restore_last(app);
    view! { <Shell /> }
}

fn main() {
    console_error_panic_hook::set_once();
    if let Some(boot) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("boot"))
    {
        boot.remove();
    }
    js::register_sw();
    leptos::mount::mount_to_body(App);
}
