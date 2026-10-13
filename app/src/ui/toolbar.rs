//! Second row: panel toggles, file/history buttons and the tools of the
//! active editor, laid out like EasyEDA's toolbar but without promotions.

use crate::commands;
use crate::icons::Icon;
use crate::state::{use_app, Tool};
use leptos::prelude::*;

fn tool_of(id: &str) -> Option<Tool> {
    Some(match id {
        "tool.select" => Tool::Select,
        "tool.wire" => Tool::Wire,
        "tool.label" => Tool::Label,
        "tool.power" => Tool::Power,
        "tool.ground" => Tool::Ground,
        "tool.noconnect" => Tool::NoConnect,
        "tool.text" => Tool::Text,
        "tool.track" => Tool::Track,
        "tool.via" => Tool::Via,
        "tool.pour" => Tool::Pour,
        "tool.outline" => Tool::Outline,
        "tool.line" => Tool::Line,
        "tool.measure" => Tool::Measure,
        "sym.pin" => Tool::Pin,
        "sym.line" => Tool::SymLine,
        "sym.rect" => Tool::SymRect,
        "sym.circle" => Tool::SymCircle,
        "fp.smd" => Tool::PadSmd,
        "fp.tht" => Tool::PadTht,
        "fp.line" => Tool::FpLine,
        "fp.rect" => Tool::FpRect,
        "fp.circle" => Tool::FpCircle,
        _ => return None,
    })
}

#[component]
fn Btn(
    id: &'static str,
    #[prop(optional)] text: bool,
    #[prop(optional)] label: Option<&'static str>,
) -> impl IntoView {
    let app = use_app();
    let c = commands::find(id).expect("command");
    let active = move || tool_of(id).is_some_and(|t| app.tool.get() == t);
    let title = move || {
        let k = commands::key_for(&app, c);
        if k.is_empty() {
            c.label.to_string()
        } else {
            format!("{} ({k})", c.label)
        }
    };
    view! {
        <button class="tb" class:active=active title=title aria-label=c.label on:click=move |_| commands::run(&app, id)>
            <Icon name=c.icon />
            {(text || label.is_some()).then(|| view! { <span class="tb-text">{label.unwrap_or(c.label)}</span> })}
        </button>
    }
}

#[component]
pub fn Toolbar() -> impl IntoView {
    let app = use_app();
    let kind = Memo::new(move |_| app.tab().kind());
    let board_layers = Memo::new(move |_| {
        let b = app.tab().board();
        app.project.with(|p| {
            b.and_then(|b| p.board(b))
                .map(|bd| bd.pcb.copper_stack())
                .unwrap_or_default()
        })
    });
    view! {
        <div class="toolbar">
            <button class="tb" class:active=move || app.show_left.get() title="Left panel (Ctrl+B)" on:click=move |_| commands::run(&app, "view.left")><Icon name="panel-left" /></button>
            <button class="tb" class:active=move || app.show_bottom.get() title="Bottom panel (Ctrl+J)" on:click=move |_| commands::run(&app, "view.bottom")><Icon name="panel-bottom" /></button>
            <button class="tb" class:active=move || app.show_right.get() title="Properties panel" on:click=move |_| commands::run(&app, "view.right")><Icon name="panel-right" /></button>
            <span class="sep"></span>
            <Btn id="file.new" />
            <Btn id="file.open" />
            <Btn id="file.save" />
            <span class="sep"></span>
            <Btn id="edit.undo" />
            <Btn id="edit.redo" />
            <span class="sep"></span>
            {move || match kind.get() {
                "sch" => view! {
                    <Btn id="tool.select" />
                    <Btn id="tool.wire" />
                    <Btn id="tool.label" />
                    <Btn id="tool.power" />
                    <Btn id="tool.ground" />
                    <Btn id="tool.noconnect" />
                    <Btn id="tool.text" />
                    <Btn id="tool.part" />
                    <span class="sep"></span>
                    <Btn id="edit.rotate" />
                    <Btn id="edit.mirror" />
                    <Btn id="edit.delete" />
                    <span class="sep"></span>
                    <Btn id="design.annotate" />
                    <Btn id="design.erc" label="ERC" />
                    <Btn id="design.update_pcb" label="Update PCB" />
                }.into_any(),
                "pcb" => view! {
                    <Btn id="tool.select" />
                    <Btn id="tool.track" />
                    <Btn id="tool.via" />
                    <Btn id="tool.pour" />
                    <Btn id="tool.outline" />
                    <Btn id="tool.line" />
                    <Btn id="tool.measure" />
                    <span class="sep"></span>
                    <Btn id="edit.rotate" />
                    <Btn id="edit.flip" />
                    <Btn id="edit.delete" />
                    <span class="sep"></span>
                    <select class="layer-select" title="Active layer (T / B)"
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            if let Some(l) = board_layers.get_untracked().into_iter().find(|l| l.name() == v) { app.active_layer.set(l) }
                        }>
                        {move || board_layers.get().into_iter().map(|l| {
                            let sel = app.active_layer.get() == l;
                            let style = format!("color: #{:06x}", l.color());
                            view! { <option value=l.name() selected=sel style=style>{l.name()}</option> }
                        }).collect_view()}
                    </select>
                    <span class="swatch" style=move || format!("background: #{:06x}", app.active_layer.get().color())></span>
                    <Btn id="pcb.rebuild" />
                    <span class="sep"></span>
                    <Btn id="design.drc" label="DRC" />
                    <Btn id="view.3d" label="3D" />
                }.into_any(),
                "symbol" => view! {
                    <Btn id="tool.select" />
                    <Btn id="sym.pin" />
                    <Btn id="sym.line" />
                    <Btn id="sym.rect" />
                    <Btn id="sym.circle" />
                    <span class="sep"></span>
                    <Btn id="edit.rotate" />
                    <Btn id="edit.delete" />
                }.into_any(),
                "footprint" => view! {
                    <Btn id="tool.select" />
                    <Btn id="fp.smd" />
                    <Btn id="fp.tht" />
                    <Btn id="fp.line" />
                    <Btn id="fp.rect" />
                    <Btn id="fp.circle" />
                    <span class="sep"></span>
                    <Btn id="edit.rotate" />
                    <Btn id="edit.delete" />
                }.into_any(),
                "3d" => view! { <Btn id="view.sch" label="Schematic" /><Btn id="view.pcb" label="PCB" /> }.into_any(),
                _ => view! { <Btn id="file.example" label="Open Example" /> }.into_any(),
            }}
            <div class="spacer"></div>
            {move || matches!(kind.get(), "sch" | "pcb" | "3d").then(|| view! { <Btn id="export.fab" label="Fabrication Files" /> })}
            <Btn id="settings.prefs" />
        </div>
    }
}
