//! Bottom panel: log and rule-check results.

use crate::icons::Icon;
use crate::state::{use_app, BottomTab, LogLevel, Tab, ViewCmd};
use easypro_core::check::Severity;
use leptos::prelude::*;

#[component]
pub fn BottomPanel() -> impl IntoView {
    let app = use_app();
    let count = move || app.issues.with(|i| i.len());
    // Which severities are listed.
    let show_warn = RwSignal::new(true);
    let show_info = RwSignal::new(true);
    view! {
        <section class="bottom" style=move || format!("height: {}px", app.settings.with(|s| s.bottom_height))>
            <div class="bottom-tabs">
                <button class:active=move || app.bottom_tab.get() == BottomTab::Log on:click=move |_| app.bottom_tab.set(BottomTab::Log)>"Log"</button>
                <button class:active=move || app.bottom_tab.get() == BottomTab::Issues on:click=move |_| app.bottom_tab.set(BottomTab::Issues)>
                    "Issues" {move || (count() > 0).then(|| view! { <span class="count">{count()}</span> })}
                </button>
                <span class="spacer"></span>
                {move || (app.bottom_tab.get() == BottomTab::Issues).then(|| view! {
                    <label class="chk small"><input type="checkbox" prop:checked=move || show_warn.get() on:change=move |ev| show_warn.set(event_target_checked(&ev)) />"Warnings"</label>
                    <label class="chk small"><input type="checkbox" prop:checked=move || show_info.get() on:change=move |ev| show_info.set(event_target_checked(&ev)) />"Info"</label>
                })}
                <span class="bt-title">{move || if app.bottom_tab.get() == BottomTab::Issues { app.issues_title.get() } else { String::new() }}</span>
                <button class="icon" title="Clear" on:click=move |_| match app.bottom_tab.get_untracked() { BottomTab::Log => app.log.set(vec![]), BottomTab::Issues => { app.issues.set(vec![]); app.issues_title.set(String::new()); } }><Icon name="trash" /></button>
                <button class="icon" title="Hide panel" on:click=move |_| app.show_bottom.set(false)><Icon name="x" /></button>
            </div>
            <div class="bottom-body">
                {move || match app.bottom_tab.get() {
                    BottomTab::Log => view! {
                        <div class="log">
                            {move || {
                                let lines = app.log.get();
                                if lines.is_empty() { return view! { <div class="tree-empty">"Messages appear here."</div> }.into_any(); }
                                lines.into_iter().rev().map(|l| {
                                    let cls = match l.level { LogLevel::Info => "info", LogLevel::Ok => "ok", LogLevel::Warn => "warn", LogLevel::Error => "error" };
                                    view! { <div class=format!("log-line {cls}")>{l.text}</div> }
                                }).collect_view().into_any()
                            }}
                        </div>
                    }.into_any(),
                    BottomTab::Issues => view! {
                        <div class="issues">
                            {move || {
                                let list = app.issues.get();
                                if list.is_empty() { return view! { <div class="tree-empty">"Run Design → ERC (schematic) or DRC (PCB) to check the design."</div> }.into_any(); }
                                list.into_iter().filter(|is| match is.severity {
                                    Severity::Error => true,
                                    Severity::Warning => show_warn.get(),
                                    Severity::Info => show_info.get(),
                                }).map(|is| {
                                    let sev = match is.severity { Severity::Error => "error", Severity::Warning => "warn", Severity::Info => "info" };
                                    let issue = is.clone();
                                    view! {
                                        <div class=format!("issue {sev}") on:click=move |_| {
                                            let tab = app.tab_untracked();
                                            if let (Some(sheet), Some(board)) = (issue.sheet, tab.board()) {
                                                app.open(Tab::Schematic { board, sheet });
                                            }
                                            app.selection.set(issue.objects.clone());
                                            if let Some(p) = issue.pos { app.view_cmd.set(Some(ViewCmd::Show(p))); }
                                        }>
                                            <span class="sev">{is.severity.label()}</span>
                                            <span class="msg">{is.message.clone()}</span>
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }}
                        </div>
                    }.into_any(),
                }}
            </div>
        </section>
    }
}
