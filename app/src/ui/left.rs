//! Left panel: project tree and the component library, with the vertical
//! tab rail EasyEDA users know.

use crate::actions;
use crate::editor::device::{footprint_svg, symbol_svg};
use crate::icons::Icon;
use crate::state::{use_app, AppState, LeftTab, Tab};
use crate::storage;
use easypro_core::library::Device;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn LeftPanel() -> impl IntoView {
    let app = use_app();
    view! {
        <aside class="left" style=move || format!("width: {}px", app.settings.with(|s| s.left_width))>
            <div class="rail">
                <button class:active=move || app.left_tab.get() == LeftTab::Projects on:click=move |_| app.left_tab.set(LeftTab::Projects)>"All Projects"</button>
                <button class:active=move || app.left_tab.get() == LeftTab::Library on:click=move |_| app.left_tab.set(LeftTab::Library)>"Common Library"</button>
            </div>
            <div class="left-body">
                {move || match app.left_tab.get() {
                    LeftTab::Projects => view! { <ProjectTree /> }.into_any(),
                    LeftTab::Library => view! { <LibraryPanel /> }.into_any(),
                }}
            </div>
        </aside>
    }
}

#[component]
fn TreeItem(
    icon: &'static str,
    #[prop(into)] label: String,
    active: Signal<bool>,
    on_open: impl Fn() + 'static,
    #[prop(optional)] depth: u8,
) -> impl IntoView {
    view! {
        <div class="tree-item" class:active=move || active.get() style=format!("padding-left: {}px", 8 + depth as u32 * 14) on:click=move |_| on_open() title=label.clone()>
            <Icon name=icon />
            <span>{label.clone()}</span>
        </div>
    }
}

#[component]
fn ProjectTree() -> impl IntoView {
    let app = use_app();
    let filter = RwSignal::new(String::new());
    let matches = move |s: &str| {
        let f = filter.get().to_lowercase();
        f.is_empty() || s.to_lowercase().contains(&f)
    };
    let is_tab = move |t: Tab| Signal::derive(move || app.tab() == t);
    view! {
        <div class="tree">
            <input class="filter" type="search" placeholder="Filter" prop:value=move || filter.get() on:input=move |ev| filter.set(event_target_value(&ev)) />
            <div class="tree-root">
                <div class="tree-head"><Icon name="folder-open" /><span>{move || app.project.with(|p| p.name.clone())}</span>
                    <button class="icon" title="Rename project" on:click=move |_| actions::rename_project(&app)><Icon name="text" /></button>
                </div>
                {move || {
                    let boards = app.project.with(|p| p.boards.iter().map(|b| (b.id, b.name.clone(), b.schematic.sheets.iter().map(|s| s.name.clone()).collect::<Vec<_>>())).collect::<Vec<_>>());
                    boards.into_iter().map(|(bid, bname, sheets)| view! {
                        <div class="tree-group">
                            <div class="tree-item group" style="padding-left: 8px"><Icon name="board" /><span>{bname}</span></div>
                            {sheets.into_iter().enumerate().filter(|(_, s)| matches(s)).map(|(i, s)| {
                                let t = Tab::Schematic { board: bid, sheet: i };
                                let t2 = t.clone();
                                view! { <TreeItem icon="schematic" label=s active=is_tab(t) on_open=move || app.open(t2.clone()) depth=1 /> }
                            }).collect_view()}
                            {matches("PCB").then(|| { let t = Tab::Pcb { board: bid }; let t2 = t.clone(); view! { <TreeItem icon="pcb" label="PCB" active=is_tab(t) on_open=move || app.open(t2.clone()) depth=1 /> } })}
                            {matches("3D View").then(|| { let t = Tab::View3d { board: bid }; let t2 = t.clone(); view! { <TreeItem icon="cube" label="3D View" active=is_tab(t) on_open=move || app.open(t2.clone()) depth=1 /> } })}
                        </div>
                    }).collect_view()
                }}
                <div class="tree-group">
                    <div class="tree-item group" style="padding-left: 8px"><Icon name="book" /><span>"Project Library"</span></div>
                    {move || {
                        let devs = app.project.with(|p| p.library.devices.values().map(|d| (d.id.clone(), d.name.clone())).collect::<Vec<_>>());
                        if devs.is_empty() {
                            return view! { <div class="tree-empty">"Parts you place are copied here so the project opens anywhere."</div> }.into_any();
                        }
                        devs.into_iter().filter(|(_, n)| matches(n)).map(|(id, name)| {
                            let t = Tab::Device { id: id.clone() };
                            let t2 = t.clone();
                            view! { <TreeItem icon="chip" label=name active=is_tab(t) on_open=move || app.open(t2.clone()) depth=1 /> }
                        }).collect_view().into_any()
                    }}
                </div>
            </div>
            <RecentProjects />
        </div>
    }
}

#[component]
pub fn RecentProjects() -> impl IntoView {
    let app = use_app();
    view! {
        <div class="recent">
            <div class="section-title">"In this browser"</div>
            {move || {
                let cur = app.key.get();
                let list = app.recent.get();
                if list.is_empty() {
                    return view! { <div class="tree-empty">"No saved projects yet."</div> }.into_any();
                }
                list.into_iter().map(|m| {
                    let key = m.key.clone();
                    let key2 = m.key.clone();
                    let current = key == cur;
                    let when = crate::js::local_time(&m.modified);
                    view! {
                        <div class="recent-item" class:active=current>
                            <button class="link" on:click=move |_| actions::open_recent(&app, key.clone())>
                                <Icon name="folder" />
                                <span class="rname">{m.name.clone()}</span>
                            </button>
                            <span class="rmeta">{format!("{} parts · {when}", m.parts)}</span>
                            <button class="icon" title="Remove from this browser" disabled=current on:click=move |_| delete_recent(&app, key2.clone())><Icon name="trash" /></button>
                        </div>
                    }
                }).collect_view().into_any()
            }}
        </div>
    }
}

fn delete_recent(app: &AppState, key: String) {
    let ok = web_sys::window().and_then(|w| w.confirm_with_message("Remove this project and its version history from this browser? Files you saved to disk are not affected.").ok()).unwrap_or(false);
    if !ok {
        return;
    }
    let app = *app;
    spawn_local(async move {
        let _ = storage::delete_local(&key).await;
        app.recent.set(storage::list_local().await);
    });
}

/// Built-in plus project devices, project first.
fn all_devices(app: &AppState) -> Vec<(Device, bool)> {
    let mut v: Vec<(Device, bool)> = app.project.with(|p| {
        p.library
            .devices
            .values()
            .map(|d| (d.clone(), true))
            .collect()
    });
    for d in easypro_core::builtin::library().devices.values() {
        if !v.iter().any(|(x, _)| x.id == d.id) {
            v.push((d.clone(), false));
        }
    }
    v
}

#[component]
fn LibraryPanel() -> impl IntoView {
    let app = use_app();
    let category = RwSignal::new(String::new());
    let results = Memo::new(move |_| {
        let q = app.library_query.get().to_lowercase();
        let cat = category.get();
        let terms: Vec<String> = q.split_whitespace().map(String::from).collect();
        let mut v: Vec<(Device, bool)> = all_devices(&app)
            .into_iter()
            .filter(|(d, _)| cat.is_empty() || d.category == cat)
            .filter(|(d, _)| {
                let hay = format!(
                    "{} {} {} {} {} {}",
                    d.name,
                    d.value,
                    d.category,
                    d.description,
                    d.footprint.as_deref().unwrap_or(""),
                    d.attributes.values().cloned().collect::<Vec<_>>().join(" ")
                )
                .to_lowercase();
                terms.iter().all(|t| hay.contains(t.as_str()))
            })
            .collect();
        v.sort_by(|a, b| {
            a.0.category
                .cmp(&b.0.category)
                .then(a.0.name.cmp(&b.0.name))
        });
        v
    });
    let categories = Memo::new(move |_| {
        let mut c: Vec<String> = all_devices(&app)
            .into_iter()
            .map(|(d, _)| d.category)
            .filter(|c| !c.is_empty())
            .collect();
        c.sort();
        c.dedup();
        c
    });
    let picked = Memo::new(move |_| {
        app.library_pick.get().and_then(|id| {
            all_devices(&app)
                .into_iter()
                .find(|(d, _)| d.id == id)
                .map(|x| x.0)
        })
    });
    view! {
        <div class="library">
            <input id="library-search" class="filter" type="search" placeholder="Search parts: 0603 10k, esp32, usb…"
                prop:value=move || app.library_query.get()
                on:input=move |ev| app.library_query.set(event_target_value(&ev))
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        if let Some((d, _)) = results.get_untracked().first() { app.library_pick.set(Some(d.id.clone())); }
                    }
                } />
            <div class="chips">
                <button class="chip-btn" class:on=move || category.get().is_empty() on:click=move |_| category.set(String::new())>"All"</button>
                {move || categories.get().into_iter().map(|c| {
                    let c2 = c.clone();
                    let c3 = c.clone();
                    view! { <button class="chip-btn" class:on=move || category.get() == c2 on:click=move |_| category.set(c3.clone())>{c}</button> }
                }).collect_view()}
            </div>
            <div class="results">
                {move || {
                    let r = results.get();
                    if r.is_empty() {
                        return view! { <div class="tree-empty">"No parts match. Make your own with Tools → New Device or the IC Symbol Wizard."</div> }.into_any();
                    }
                    r.into_iter().map(|(d, in_project)| {
                        let id = d.id.clone();
                        let id2 = d.id.clone();
                        let id3 = d.id.clone();
                        let fp = d.footprint.clone().unwrap_or_default().trim_start_matches("fp:").to_string();
                        view! {
                            <div class="part" class:active=move || app.library_pick.get().as_deref() == Some(id.as_str())
                                on:click=move |_| app.library_pick.set(Some(id2.clone()))
                                on:dblclick=move |_| actions::place_device(&app, &id3)
                                title="Click to preview, double-click to place">
                                <div class="part-name">{d.name.clone()}{in_project.then(|| view! { <span class="badge">"project"</span> })}</div>
                                <div class="part-meta">{format!("{} · {}", d.value, fp)}</div>
                            </div>
                        }
                    }).collect_view().into_any()
                }}
            </div>
            {move || picked.get().map(|d| {
                let id = d.id.clone();
                let id2 = d.id.clone();
                view! {
                    <div class="part-preview">
                        <div class="pp-head">
                            <strong>{d.name.clone()}</strong>
                            <span class="hint">{d.description.clone()}</span>
                        </div>
                        <div class="pp-images">
                            <div class="preview light" inner_html=symbol_svg(&app, &d.symbol)></div>
                            <div class="preview dark" inner_html=footprint_svg(&app, d.footprint.as_deref().unwrap_or(""))></div>
                        </div>
                        <dl class="pp-attrs">
                            {d.attributes.iter().map(|(k, v)| view! { <dt>{k.clone()}</dt><dd>{v.clone()}</dd> }).collect_view()}
                        </dl>
                        <div class="row">
                            <button class="primary" on:click=move |_| actions::place_device(&app, &id)><Icon name="plus" />"Place"</button>
                            <button on:click=move |_| {
                                let lib = easypro_core::builtin::library();
                                let id = id2.clone();
                                if app.project.with_untracked(|p| !p.library.devices.contains_key(&id)) {
                                    app.edit("Add part to project library", |p| { p.library.import_device(lib, &id); });
                                }
                                app.open(Tab::Device { id });
                            }><Icon name="chip" />"Open Device"</button>
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
