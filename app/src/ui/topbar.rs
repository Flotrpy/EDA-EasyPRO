//! Blue title bar with the classic menu row.

use crate::commands::{self, MENUS};
use crate::icons::Icon;
use crate::state::use_app;
use leptos::prelude::*;

#[component]
pub fn TopBar() -> impl IntoView {
    let app = use_app();
    view! {
        <header class="topbar">
            <div class="brand" title="EasyPRO">
                <span class="logo" inner_html=include_str!("../../assets/favicon.svg")></span>
                <span class="name">"EasyPRO"</span>
            </div>
            <nav class="menus" role="menubar">
                {MENUS.into_iter().map(|m| view! { <Menu name=m /> }).collect_view()}
            </nav>
            <div class="spacer"></div>
            <button class="search" on:click=move |_| app.palette.set(true) title="Search commands and parts">
                <Icon name="search" />
                <span>"Search commands, parts…"</span>
                <kbd>"Ctrl K"</kbd>
            </button>
            <div class="project-name" title="Project">
                {move || app.project.with(|p| p.name.clone())}
                <span class="dirty" class:on=move || app.dirty.get() title="Unsaved changes (kept in this browser)">"●"</span>
            </div>
            <button class="icon light" title="Toggle dark theme" on:click=move |_| commands::run(&app, "view.theme")>
                {move || if app.dark.get() { view! { <Icon name="sun" /> } } else { view! { <Icon name="moon" /> } }}
            </button>
            <span class="version">{concat!("v", env!("CARGO_PKG_VERSION"))}</span>
        </header>
    }
}

#[component]
fn Menu(name: &'static str) -> impl IntoView {
    let app = use_app();
    let open = move || app.menu_open.get() == Some(name);
    view! {
        <div class="menu" class:open=open>
            <button
                class="menu-title"
                role="menuitem"
                on:click=move |ev| {
                    ev.stop_propagation();
                    app.menu_open.set(if open() { None } else { Some(name) });
                }
                on:mouseenter=move |_| if app.menu_open.get_untracked().is_some() { app.menu_open.set(Some(name)) }
            >
                {name}
            </button>
            <Show when=open>
                <div class="dropdown" role="menu" on:click=|ev| ev.stop_propagation()>
                    {move || {
                        app.active.track();
                        app.tabs.track();
                        commands::menu_items(name).into_iter().map(|c| {
                            let enabled = commands::enabled(&app, c);
                            let key = commands::key_for(&app, c);
                            let id = c.id;
                            view! {
                                <button class="item" disabled=!enabled on:click=move |_| commands::run(&app, id)>
                                    <Icon name=c.icon />
                                    <span class="label">{c.label}</span>
                                    <kbd>{key}</kbd>
                                </button>
                            }
                        }).collect_view()
                    }}
                </div>
            </Show>
        </div>
    }
}
