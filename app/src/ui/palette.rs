//! Command palette (Ctrl+K): fuzzy search over commands and library parts.

use crate::actions;
use crate::commands::{self, COMMANDS};
use crate::icons::Icon;
use crate::state::{use_app, Tab, ViewCmd};
use easypro_core::geom::Point;
use easypro_core::id::Id;
use leptos::html;
use leptos::prelude::*;

#[derive(Clone, PartialEq)]
enum Entry {
    Cmd(&'static str, &'static str, &'static str, String),
    Part(String, String, String),
    /// Jump to a placed part: tab, object, position, label.
    Goto(Tab, Id, Point, String),
}

/// Subsequence match score; lower is better, None when not matching.
fn score(hay: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    let hay = hay.to_lowercase();
    if let Some(i) = hay.find(needle) {
        return Some(i);
    }
    let mut pos = 0;
    let mut gaps = 0;
    for c in needle.chars() {
        let rest = &hay[pos..];
        let i = rest.find(c)?;
        gaps += i;
        pos += i + c.len_utf8();
    }
    Some(100 + gaps)
}

#[component]
pub fn Palette() -> impl IntoView {
    let app = use_app();
    let query = RwSignal::new(String::new());
    let cursor = RwSignal::new(0usize);
    let input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        if let Some(i) = input.get() {
            let _ = i.focus();
        }
    });
    let entries = Memo::new(move |_| {
        let q = query.get().to_lowercase();
        let q = q.trim().to_string();
        let mut out: Vec<(usize, Entry)> = COMMANDS
            .iter()
            .filter(|c| !c.menu.is_empty() && commands::enabled(&app, c))
            .filter_map(|c| {
                score(&format!("{} {}", c.menu, c.label), &q).map(|s| {
                    (
                        s,
                        Entry::Cmd(c.id, c.label, c.icon, commands::key_for(&app, c)),
                    )
                })
            })
            .collect();
        if !q.is_empty() {
            let mut gotos = Vec::new();
            app.project.with_untracked(|p| {
                for b in &p.boards {
                    for (si, c) in b.schematic.components() {
                        if c.designator.to_lowercase().starts_with(&q) {
                            let tab = Tab::Schematic {
                                board: b.id,
                                sheet: si,
                            };
                            let label =
                                format!("Go to {} ({}) in schematic", c.designator, c.value);
                            gotos.push(Entry::Goto(tab, c.id, c.pos, label));
                        }
                    }
                    for c in &b.pcb.components {
                        if c.designator.to_lowercase().starts_with(&q) {
                            let label = format!("Go to {} in PCB", c.designator);
                            gotos.push(Entry::Goto(Tab::Pcb { board: b.id }, c.id, c.pos, label));
                        }
                    }
                }
            });
            out.extend(gotos.into_iter().take(10).map(|e| (5, e)));
        }
        if q.len() >= 2 {
            for d in easypro_core::builtin::library()
                .search(&q)
                .into_iter()
                .take(8)
            {
                out.push((
                    50,
                    Entry::Part(
                        d.id.clone(),
                        d.name.clone(),
                        d.footprint.clone().unwrap_or_default(),
                    ),
                ));
            }
        }
        out.sort_by_key(|(s, _)| *s);
        out.into_iter().map(|(_, e)| e).take(40).collect::<Vec<_>>()
    });
    let run = move |e: &Entry| {
        app.palette.set(false);
        match e {
            Entry::Cmd(id, ..) => commands::run(&app, id),
            Entry::Part(id, ..) => actions::place_device(&app, id),
            Entry::Goto(tab, id, pos, _) => {
                app.open(tab.clone());
                app.selection.set(vec![*id]);
                app.view_cmd.set(Some(ViewCmd::Show(*pos)));
            }
        }
    };
    view! {
        <div class="overlay" on:click=move |_| app.palette.set(false)>
            <div class="palette" on:click=|ev| ev.stop_propagation()>
                <div class="palette-input">
                    <Icon name="search" />
                    <input node_ref=input type="text" placeholder="Type a command or a part (e.g. route, drc, 0805 cap)…"
                        prop:value=move || query.get()
                        on:input=move |ev| { query.set(event_target_value(&ev)); cursor.set(0); }
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            let n = entries.with_untracked(|e| e.len());
                            match ev.key().as_str() {
                                "ArrowDown" => { ev.prevent_default(); cursor.update(|c| *c = (*c + 1).min(n.saturating_sub(1))); }
                                "ArrowUp" => { ev.prevent_default(); cursor.update(|c| *c = c.saturating_sub(1)); }
                                "Enter" => { ev.prevent_default(); if let Some(e) = entries.with_untracked(|e| e.get(cursor.get_untracked()).cloned()) { run(&e); } }
                                "Escape" => { ev.prevent_default(); app.palette.set(false); }
                                _ => {}
                            }
                            ev.stop_propagation();
                        } />
                </div>
                <div class="palette-list">
                    {move || entries.get().into_iter().enumerate().map(|(i, e)| {
                        let e2 = e.clone();
                        let (icon, label, hint) = match &e {
                            Entry::Cmd(_, l, ic, k) => (*ic, l.to_string(), k.clone()),
                            Entry::Part(_, name, fp) => ("chip", format!("Place {name}"), fp.trim_start_matches("fp:").to_string()),
                            Entry::Goto(tab, _, _, label) => {
                                let icon = if matches!(tab, Tab::Pcb { .. }) { "pcb" } else { "schematic" };
                                (icon, label.clone(), String::new())
                            }
                        };
                        view! {
                            <button class="palette-item" class:active=move || cursor.get() == i on:mouseenter=move |_| cursor.set(i) on:click=move |_| run(&e2)>
                                <Icon name=icon />
                                <span class="label">{label}</span>
                                <kbd>{hint}</kbd>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}
