//! Right panel: properties of the selection, plus layers and nets on boards.

use crate::actions;
use crate::icons::Icon;
use crate::state::{use_app, AppState, Tab};
use easypro_core::geom::normalize_angle;
use easypro_core::id::Id;
use easypro_core::layer::{Layer, Side};
use easypro_core::library::{PadKind, PadShape, PinKind};
use easypro_core::pick::{pcb_kind, sch_kind, PcbKind, SchKind};
use easypro_core::project::Project;
use easypro_core::schematic::{PaperSize, PortStyle};
use easypro_core::units::parse_length;
use leptos::prelude::*;

/// A labelled text input that commits on change.
#[component]
fn TextProp(
    label: &'static str,
    value: String,
    on_set: impl Fn(String) + 'static,
) -> impl IntoView {
    view! {
        <label class="prop">
            <span>{label}</span>
            <input type="text" prop:value=value on:change=move |ev| on_set(event_target_value(&ev)) />
        </label>
    }
}

/// A length input in the user's unit; accepts "8mil", "0.2mm" etc.
#[component]
fn LenProp(label: &'static str, nm: i64, on_set: impl Fn(i64) + 'static) -> impl IntoView {
    let app = use_app();
    let unit = app.settings.with_untracked(|s| s.unit);
    view! {
        <label class="prop">
            <span>{label}</span>
            <input type="text" prop:value=unit.format(nm) on:change=move |ev| {
                match parse_length(&event_target_value(&ev), unit) {
                    Some(v) => on_set(v),
                    None => app.warn("Enter a length such as 0.25, 10mil or 0.1in"),
                }
            } />
            <em>{unit.label()}</em>
        </label>
    }
}

#[component]
fn NumProp(label: &'static str, value: f64, on_set: impl Fn(f64) + 'static) -> impl IntoView {
    view! {
        <label class="prop">
            <span>{label}</span>
            <input type="number" prop:value=format!("{value}") on:change=move |ev| { if let Ok(v) = event_target_value(&ev).parse::<f64>() { on_set(v) } } />
        </label>
    }
}

#[component]
fn CheckProp(label: &'static str, value: bool, on_set: impl Fn(bool) + 'static) -> impl IntoView {
    view! {
        <label class="prop check">
            <input type="checkbox" prop:checked=value on:change=move |ev| on_set(event_target_checked(&ev)) />
            <span>{label}</span>
        </label>
    }
}

#[component]
fn SelectProp(
    label: &'static str,
    options: Vec<(String, String)>,
    value: String,
    on_set: impl Fn(String) + 'static,
) -> impl IntoView {
    view! {
        <label class="prop">
            <span>{label}</span>
            <select on:change=move |ev| on_set(event_target_value(&ev))>
                {options.into_iter().map(|(v, l)| { let sel = v == value; view! { <option value=v selected=sel>{l}</option> } }).collect_view()}
            </select>
        </label>
    }
}

fn rotations() -> Vec<(String, String)> {
    ["0", "90", "180", "270"]
        .iter()
        .map(|r| (r.to_string(), format!("{r}°")))
        .collect()
}

fn sheet_edit(
    app: &AppState,
    b: Id,
    s: usize,
    label: &str,
    f: impl FnOnce(&mut easypro_core::schematic::Sheet),
) {
    app.edit(label, |p| {
        if let Some(sh) = p.board_mut(b).and_then(|bd| bd.schematic.sheets.get_mut(s)) {
            f(sh);
        }
    });
}

fn pcb_edit(
    app: &AppState,
    b: Id,
    label: &str,
    rebuild: bool,
    f: impl FnOnce(&mut easypro_core::pcb::Pcb),
) {
    app.edit(label, |p| {
        let lib = p.library.clone();
        if let Some(bd) = p.board_mut(b) {
            f(&mut bd.pcb);
            if rebuild {
                bd.pcb.rebuild_pours(&lib);
            }
        }
    });
}

#[component]
pub fn RightPanel() -> impl IntoView {
    let app = use_app();
    view! {
        <aside class="right" style=move || format!("width: {}px", app.settings.with(|s| s.right_width))>
            <div class="panel-title"><Icon name="settings" />"Properties"</div>
            <div class="props">
                {move || {
                    app.rev.track();
                    let tab = app.tab();
                    let sel = app.selection.get();
                    app.settings.with(|s| s.unit);
                    match tab {
                        Tab::Schematic { board, sheet } => sch_props(&app, board, sheet, &sel).into_any(),
                        Tab::Pcb { board } => pcb_props(&app, board, &sel).into_any(),
                        Tab::Symbol { id } => sym_props(&app, &id, &sel).into_any(),
                        Tab::Footprint { id } => fp_props(&app, &id, &sel).into_any(),
                        Tab::Start => view! { <p class="hint">"Open or create a project to start. Everything stays on this computer."</p> }.into_any(),
                        Tab::View3d { .. } => view! { <p class="hint">"3D preview of the board. Component bodies come from each device's 3D settings."</p> }.into_any(),
                        Tab::Device { .. } => view! { <p class="hint">"Edit the device on the page. Changes apply to every placed copy in this project."</p> }.into_any(),
                    }
                }}
            </div>
            {move || matches!(app.tab(), Tab::Pcb { .. }).then(|| view! { <LayersPanel /> <NetsPanel /> })}
        </aside>
    }
}

fn sch_props(app: &AppState, b: Id, s: usize, sel: &[Id]) -> impl IntoView {
    let app = *app;
    let data = app.project.with_untracked(|p: &Project| {
        let bd = p.board(b)?;
        let sh = bd.schematic.sheets.get(s)?.clone();
        Some((
            sh,
            p.library.clone(),
            easypro_core::netlist::connectivity(&bd.schematic, &p.library),
        ))
    });
    let Some((sh, lib, conn)) = data else {
        return view! { <p>"Sheet not found"</p> }.into_any();
    };
    if sel.len() > 1 {
        return view! {
            <p class="hint">{format!("{} objects selected", sel.len())}</p>
            <div class="row">
                <button on:click=move |_| actions::rotate(&app)><Icon name="rotate" />"Rotate"</button>
                <button on:click=move |_| actions::mirror(&app)><Icon name="mirror" />"Mirror"</button>
                <button on:click=move |_| actions::delete(&app)><Icon name="trash" />"Delete"</button>
            </div>
        }
        .into_any();
    }
    let Some(&id) = sel.first() else {
        let papers: Vec<(String, String)> = PaperSize::ALL
            .iter()
            .map(|p| (p.label().to_string(), p.label().to_string()))
            .collect();
        let parts = sh.components.len();
        let nets = conn.nets.len();
        return view! {
            <h4>"Sheet"</h4>
            <TextProp label="Name" value=sh.name.clone() on_set=move |v| sheet_edit(&app, b, s, "Rename sheet", |sh| sh.name = v) />
            <SelectProp label="Paper" options=papers value=sh.paper.label().to_string() on_set=move |v| sheet_edit(&app, b, s, "Paper size", |sh| sh.paper = PaperSize::ALL.into_iter().find(|p| p.label() == v).unwrap_or_default()) />
            <p class="hint">{format!("{parts} parts, {nets} nets on this board's schematic.")}</p>
            <p class="hint">"Tip: place parts from the Common Library (Shift+F), wire with W, label nets with N."</p>
        }
        .into_any();
    };
    match sch_kind(&sh, id) {
        Some(SchKind::Component) => {
            let c = sh.component(id).cloned().unwrap();
            let dev = lib.devices.get(&c.device).cloned();
            let dev_id = c.device.clone();
            let attr_keys = ["Manufacturer", "MPN", "LCSC"];
            view! {
                <h4>{dev.as_ref().map(|d| d.name.clone()).unwrap_or_else(|| c.device.clone())}</h4>
                <TextProp label="Designator" value=c.designator.clone() on_set=move |v| sheet_edit(&app, b, s, "Designator", |sh| if let Some(c) = sh.component_mut(id) { c.designator = v }) />
                <TextProp label="Value" value=c.value.clone() on_set=move |v| sheet_edit(&app, b, s, "Value", |sh| if let Some(c) = sh.component_mut(id) { c.value = v }) />
                <SelectProp label="Rotation" options=rotations() value=format!("{}", c.rotation) on_set=move |v| sheet_edit(&app, b, s, "Rotation", |sh| if let Some(c) = sh.component_mut(id) { c.rotation = normalize_angle(v.parse().unwrap_or(0.0)) }) />
                <CheckProp label="Mirrored" value=c.mirror on_set=move |v| sheet_edit(&app, b, s, "Mirror", |sh| if let Some(c) = sh.component_mut(id) { c.mirror = v }) />
                <CheckProp label="Exclude from BOM" value=c.exclude_from_bom on_set=move |v| sheet_edit(&app, b, s, "BOM flag", |sh| if let Some(c) = sh.component_mut(id) { c.exclude_from_bom = v }) />
                <h4>"Part numbers"</h4>
                {attr_keys.into_iter().map(|k| {
                    let val = c.attr(k, &lib).unwrap_or("").to_string();
                    view! { <TextProp label=k value=val on_set=move |v| sheet_edit(&app, b, s, "Attribute", |sh| if let Some(c) = sh.component_mut(id) { c.attributes.insert(k.to_string(), v); }) /> }
                }).collect_view()}
                <div class="row">
                    <button on:click=move |_| app.open(Tab::Device { id: dev_id.clone() })><Icon name="chip" />"Edit Device"</button>
                </div>
                <h4>"Pins"</h4>
                <table class="kv small">
                    <tbody>
                    {lib.devices.get(&c.device).and_then(|d| lib.symbols.get(&d.symbol)).map(|sym| sym.pins.iter().map(|pin| {
                        let net = conn.net_of_pin(c.id, &pin.number).map(|n| n.name.clone()).unwrap_or_else(|| "—".into());
                        view! { <tr><td>{pin.number.clone()}</td><td>{pin.name.clone()}</td><td class="net">{net}</td></tr> }
                    }).collect_view())}
                    </tbody>
                </table>
            }
            .into_any()
        }
        Some(SchKind::Label) => {
            let l = sh.labels.iter().find(|l| l.id == id).cloned().unwrap();
            view! {
                <h4>"Net Label"</h4>
                <TextProp label="Net name" value=l.name.clone() on_set=move |v| sheet_edit(&app, b, s, "Rename label", |sh| if let Some(l) = sh.labels.iter_mut().find(|l| l.id == id) { l.name = v }) />
                <SelectProp label="Rotation" options=vec![("0".into(), "Horizontal".into()), ("90".into(), "Vertical".into())] value=format!("{}", l.rotation) on_set=move |v| sheet_edit(&app, b, s, "Rotate label", |sh| if let Some(l) = sh.labels.iter_mut().find(|l| l.id == id) { l.rotation = v.parse().unwrap_or(0.0) }) />
            }
            .into_any()
        }
        Some(SchKind::Port) => {
            let pt = sh.ports.iter().find(|l| l.id == id).cloned().unwrap();
            let styles = vec![("power".to_string(), "Power".to_string()), ("ground".into(), "Ground".into()), ("port".into(), "Port".into())];
            let cur = match pt.style {
                PortStyle::Power => "power",
                PortStyle::Ground => "ground",
                PortStyle::Port => "port",
            };
            view! {
                <h4>"Power / Port"</h4>
                <TextProp label="Net name" value=pt.name.clone() on_set=move |v| sheet_edit(&app, b, s, "Rename port", |sh| if let Some(l) = sh.ports.iter_mut().find(|l| l.id == id) { l.name = v }) />
                <SelectProp label="Style" options=styles value=cur.to_string() on_set=move |v| sheet_edit(&app, b, s, "Port style", |sh| if let Some(l) = sh.ports.iter_mut().find(|l| l.id == id) {
                    l.style = match v.as_str() { "ground" => PortStyle::Ground, "port" => PortStyle::Port, _ => PortStyle::Power };
                }) />
                <div class="row">
                    {["VCC", "+5V", "+3V3", "+12V", "GND", "AGND"].into_iter().map(|n| view! {
                        <button class="chip-btn" on:click=move |_| sheet_edit(&app, b, s, "Rename port", |sh| if let Some(l) = sh.ports.iter_mut().find(|l| l.id == id) { l.name = n.to_string() })>{n}</button>
                    }).collect_view()}
                </div>
            }
            .into_any()
        }
        Some(SchKind::Wire) => {
            let net = conn.wire_net.get(&id).map(|&i| conn.nets[i].name.clone()).unwrap_or_else(|| "(not connected to any pin)".into());
            let n = sh.wires.iter().find(|w| w.id == id).map(|w| w.points.len().saturating_sub(1)).unwrap_or(0);
            view! { <h4>"Wire"</h4><p class="prop-ro"><span>"Net"</span><b>{net}</b></p><p class="prop-ro"><span>"Segments"</span><b>{n}</b></p> }.into_any()
        }
        Some(SchKind::Annotation) => {
            let a = sh.annotations.iter().find(|a| a.id == id).cloned().unwrap();
            match a.shape {
                easypro_core::shape::Shape::Text { text, .. } => view! {
                    <h4>"Text"</h4>
                    <TextProp label="Text" value=text on_set=move |v| sheet_edit(&app, b, s, "Edit text", |sh| if let Some(a) = sh.annotations.iter_mut().find(|a| a.id == id) {
                        if let easypro_core::shape::Shape::Text { text, .. } = &mut a.shape { *text = v; }
                    }) />
                }
                .into_any(),
                _ => view! { <h4>"Graphic"</h4> }.into_any(),
            }
        }
        Some(SchKind::NoConnect) => view! { <h4>"No-Connect Flag"</h4><p class="hint">"Tells ERC this pin is unused on purpose."</p> }.into_any(),
        None => view! { <p class="hint">"Nothing selected"</p> }.into_any(),
    }
}

fn layer_options(p: &easypro_core::pcb::Pcb, copper_only: bool) -> Vec<(String, String)> {
    Layer::all_for(p.copper_layers)
        .into_iter()
        .filter(|l| !copper_only || (l.is_copper() && *l != Layer::Multi))
        .map(|l| (l.name(), l.name()))
        .collect()
}

fn layer_by_name(p: &easypro_core::pcb::Pcb, name: &str) -> Option<Layer> {
    Layer::all_for(p.copper_layers)
        .into_iter()
        .find(|l| l.name() == name)
}

fn pcb_props(app: &AppState, b: Id, sel: &[Id]) -> impl IntoView {
    let app = *app;
    let data = app
        .project
        .with_untracked(|p| p.board(b).map(|bd| (bd.pcb.clone(), p.library.clone())));
    let Some((pcb, lib)) = data else {
        return view! { <p>"Board not found"</p> }.into_any();
    };
    if sel.len() > 1 {
        return view! {
            <p class="hint">{format!("{} objects selected", sel.len())}</p>
            <div class="row">
                <button on:click=move |_| actions::rotate(&app)><Icon name="rotate" />"Rotate"</button>
                <button on:click=move |_| actions::flip(&app)><Icon name="flip" />"Flip"</button>
                <button on:click=move |_| actions::delete(&app)><Icon name="trash" />"Delete"</button>
            </div>
        }
        .into_any();
    }
    let nets: Vec<(String, String)> = std::iter::once((String::new(), "(no net)".to_string()))
        .chain(pcb.nets().into_iter().map(|n| (n.clone(), n)))
        .collect();
    let Some(&id) = sel.first() else {
        let size = pcb
            .outline()
            .map(easypro_core::geom::Rect::from_points)
            .map(|r| format!("{} x {}", app.fmt_len(r.width()), app.fmt_len(r.height())))
            .unwrap_or_else(|| "no outline".into());
        return view! {
            <h4>"Board"</h4>
            <p class="prop-ro"><span>"Size"</span><b>{size}</b></p>
            <p class="prop-ro"><span>"Copper layers"</span><b>{pcb.copper_layers}</b></p>
            <p class="prop-ro"><span>"Thickness"</span><b>{app.fmt_len(pcb.thickness)}</b></p>
            <p class="prop-ro"><span>"Parts / tracks / vias"</span><b>{format!("{} / {} / {}", pcb.components.len(), pcb.tracks.len(), pcb.vias.len())}</b></p>
            <div class="row">
                <button on:click=move |_| actions::board_setup(&app)><Icon name="board" />"Board Setup"</button>
                <button on:click=move |_| actions::rules(&app)><Icon name="settings" />"Design Rules"</button>
            </div>
        }
        .into_any();
    };
    match pcb_kind(&pcb, id) {
        Some(PcbKind::Component) => {
            let c = pcb.component(id).cloned().unwrap();
            let fp = lib
                .footprints
                .get(&c.footprint)
                .map(|f| f.name.clone())
                .unwrap_or_else(|| c.footprint.clone());
            let link = c.link;
            view! {
                <h4>{format!("{} · {}", c.designator, fp)}</h4>
                <TextProp label="Designator" value=c.designator.clone() on_set=move |v| pcb_edit(&app, b, "Designator", false, |p| if let Some(c) = p.component_mut(id) { c.designator = v }) />
                <LenProp label="X" nm=c.pos.x on_set=move |v| pcb_edit(&app, b, "Move", true, |p| if let Some(c) = p.component_mut(id) { c.pos.x = v }) />
                <LenProp label="Y" nm=c.pos.y on_set=move |v| pcb_edit(&app, b, "Move", true, |p| if let Some(c) = p.component_mut(id) { c.pos.y = v }) />
                <NumProp label="Rotation °" value=c.rotation on_set=move |v| pcb_edit(&app, b, "Rotation", true, |p| if let Some(c) = p.component_mut(id) { c.rotation = normalize_angle(v) }) />
                <SelectProp label="Side" options=vec![("top".into(), "Top".into()), ("bottom".into(), "Bottom".into())] value=if c.side == Side::Top { "top".into() } else { "bottom".into() }
                    on_set=move |v| pcb_edit(&app, b, "Side", true, |p| if let Some(c) = p.component_mut(id) { let want = if v == "top" { Side::Top } else { Side::Bottom }; if c.side != want { c.side = want; c.rotation = normalize_angle(-c.rotation); } }) />
                <CheckProp label="Locked" value=c.locked on_set=move |v| pcb_edit(&app, b, "Lock", false, |p| if let Some(c) = p.component_mut(id) { c.locked = v }) />
                <CheckProp label="Show designator" value=c.show_designator on_set=move |v| pcb_edit(&app, b, "Designator visibility", false, |p| if let Some(c) = p.component_mut(id) { c.show_designator = v }) />
                <p class="prop-ro"><span>"Value"</span><b>{c.value.clone()}</b></p>
                {link.is_none().then(|| view! { <p class="hint warn">"Not linked to a schematic part."</p> })}
                <h4>"Pads"</h4>
                <table class="kv small"><tbody>
                    {c.pad_nets.iter().map(|(pad, net)| {
                        let n = net.clone();
                        view! { <tr><td>{pad.clone()}</td><td class="net link" on:click=move |_| app.highlight_net.set(Some(n.clone()))>{net.clone()}</td></tr> }
                    }).collect_view()}
                </tbody></table>
            }
            .into_any()
        }
        Some(PcbKind::Track) => {
            let t = pcb.tracks.iter().find(|t| t.id == id).cloned().unwrap();
            let net = t.net.clone();
            view! {
                <h4>"Track"</h4>
                <SelectProp label="Layer" options=layer_options(&pcb, true) value=t.layer.name() on_set=move |v| pcb_edit(&app, b, "Track layer", true, |p| { let l = layer_by_name(p, &v); if let (Some(t), Some(l)) = (p.tracks.iter_mut().find(|t| t.id == id), l) { t.layer = l } }) />
                <LenProp label="Width" nm=t.width on_set=move |v| pcb_edit(&app, b, "Track width", true, |p| if let Some(t) = p.tracks.iter_mut().find(|t| t.id == id) { t.width = v }) />
                <SelectProp label="Net" options=nets value=t.net.clone() on_set=move |v| pcb_edit(&app, b, "Track net", true, |p| if let Some(t) = p.tracks.iter_mut().find(|t| t.id == id) { t.net = v }) />
                <p class="prop-ro"><span>"Length"</span><b>{app.fmt_len(t.length() as i64)}</b></p>
                <div class="row"><button on:click=move |_| app.highlight_net.set(Some(net.clone()))><Icon name="eye" />"Highlight Net"</button></div>
            }
            .into_any()
        }
        Some(PcbKind::Via) => {
            let v = pcb.vias.iter().find(|t| t.id == id).cloned().unwrap();
            view! {
                <h4>"Via"</h4>
                <LenProp label="Diameter" nm=v.diameter on_set=move |x| pcb_edit(&app, b, "Via size", true, |p| if let Some(v) = p.vias.iter_mut().find(|t| t.id == id) { v.diameter = x }) />
                <LenProp label="Drill" nm=v.drill on_set=move |x| pcb_edit(&app, b, "Via drill", true, |p| if let Some(v) = p.vias.iter_mut().find(|t| t.id == id) { v.drill = x }) />
                <SelectProp label="Net" options=nets value=v.net.clone() on_set=move |x| pcb_edit(&app, b, "Via net", true, |p| if let Some(v) = p.vias.iter_mut().find(|t| t.id == id) { v.net = x }) />
            }
            .into_any()
        }
        Some(PcbKind::Pour) => {
            let po = pcb.pours.iter().find(|t| t.id == id).cloned().unwrap();
            let area: f64 = po.filled.iter().map(|r| r.area()).sum::<f64>() / 1e12;
            view! {
                <h4>"Copper Area"</h4>
                <SelectProp label="Layer" options=layer_options(&pcb, true) value=po.layer.name() on_set=move |v| pcb_edit(&app, b, "Pour layer", true, |p| { let l = layer_by_name(p, &v); if let (Some(t), Some(l)) = (p.pours.iter_mut().find(|t| t.id == id), l) { t.layer = l } }) />
                <SelectProp label="Net" options=nets value=po.net.clone() on_set=move |v| pcb_edit(&app, b, "Pour net", true, |p| if let Some(t) = p.pours.iter_mut().find(|t| t.id == id) { t.net = v }) />
                <LenProp label="Clearance" nm=po.clearance.unwrap_or(pcb.rules.clearance) on_set=move |v| pcb_edit(&app, b, "Pour clearance", true, |p| if let Some(t) = p.pours.iter_mut().find(|t| t.id == id) { t.clearance = Some(v) }) />
                <CheckProp label="Remove dead copper" value=po.remove_islands on_set=move |v| pcb_edit(&app, b, "Pour islands", true, |p| if let Some(t) = p.pours.iter_mut().find(|t| t.id == id) { t.remove_islands = v }) />
                <p class="prop-ro"><span>"Filled area"</span><b>{format!("{area:.1} mm²")}</b></p>
            }
            .into_any()
        }
        Some(PcbKind::Graphic) => {
            let g = pcb.graphics.iter().find(|t| t.id == id).cloned().unwrap();
            let text = match &g.shape {
                easypro_core::shape::Shape::Text { text, size, .. } => Some((text.clone(), *size)),
                _ => None,
            };
            let set_text = move |f: Box<dyn FnOnce(&mut String, &mut i64)>| {
                pcb_edit(&app, b, "Edit text", false, |p| {
                    if let Some(easypro_core::shape::Shape::Text { text, size, .. }) = p
                        .graphics
                        .iter_mut()
                        .find(|t| t.id == id)
                        .map(|g| &mut g.shape)
                    {
                        f(text, size);
                    }
                })
            };
            view! {
                <h4>{if text.is_some() { "Text" } else { "Graphic" }}</h4>
                <SelectProp label="Layer" options=layer_options(&pcb, false) value=g.layer.name() on_set=move |v| pcb_edit(&app, b, "Graphic layer", false, |p| { let l = layer_by_name(p, &v); if let (Some(t), Some(l)) = (p.graphics.iter_mut().find(|t| t.id == id), l) { t.layer = l } }) />
                {text.map(|(t, size)| view! {
                    <TextProp label="Text" value=t on_set=move |v| set_text(Box::new(move |t, _| *t = v)) />
                    <LenProp label="Height" nm=size on_set=move |v| set_text(Box::new(move |_, s| *s = v.max(100_000))) />
                })}
            }
            .into_any()
        }
        None => view! { <p class="hint">"Nothing selected"</p> }.into_any(),
    }
}

fn pin_kinds() -> Vec<(String, String)> {
    PinKind::ALL
        .iter()
        .map(|k| (format!("{k:?}"), k.label().to_string()))
        .collect()
}

fn sym_props(app: &AppState, id: &str, sel: &[Id]) -> impl IntoView {
    use crate::editor::symbol::{edit_symbol, GFX};
    let app = *app;
    let Some(sym) = app
        .project
        .with_untracked(|p| p.library.symbols.get(id).cloned())
    else {
        return view! { <p>"Symbol not found"</p> }.into_any();
    };
    let sid = StoredValue::new(id.to_string());
    match sel {
        [one] if one.0 < GFX => {
            let i = one.0 as usize;
            let Some(pin) = sym.pins.get(i).cloned() else { return view! { <p></p> }.into_any() };
            view! {
                <h4>"Pin"</h4>
                <TextProp label="Number" value=pin.number.clone() on_set=move |v| edit_symbol(&app, &sid.get_value(), "Pin number", |s| if let Some(p) = s.pins.get_mut(i) { p.number = v }) />
                <TextProp label="Name" value=pin.name.clone() on_set=move |v| edit_symbol(&app, &sid.get_value(), "Pin name", |s| if let Some(p) = s.pins.get_mut(i) { p.name = v }) />
                <SelectProp label="Type" options=pin_kinds() value=format!("{:?}", pin.kind) on_set=move |v| edit_symbol(&app, &sid.get_value(), "Pin type", |s| if let Some(p) = s.pins.get_mut(i) { p.kind = PinKind::ALL.into_iter().find(|k| format!("{k:?}") == v).unwrap_or_default() }) />
                <SelectProp label="Direction" options=vec![("0".into(), "Body to the right".into()), ("180".into(), "Body to the left".into()), ("270".into(), "Body below".into()), ("90".into(), "Body above".into())] value=format!("{}", pin.angle)
                    on_set=move |v| edit_symbol(&app, &sid.get_value(), "Pin direction", |s| if let Some(p) = s.pins.get_mut(i) { p.angle = v.parse().unwrap_or(0.0) }) />
                <LenProp label="Length" nm=pin.length on_set=move |v| edit_symbol(&app, &sid.get_value(), "Pin length", |s| if let Some(p) = s.pins.get_mut(i) { p.length = v }) />
                <CheckProp label="Hide name" value=pin.hide_name on_set=move |v| edit_symbol(&app, &sid.get_value(), "Pin name visibility", |s| if let Some(p) = s.pins.get_mut(i) { p.hide_name = v }) />
            }
            .into_any()
        }
        [] => view! {
            <h4>"Symbol"</h4>
            <TextProp label="Name" value=sym.name.clone() on_set=move |v| edit_symbol(&app, &sid.get_value(), "Rename symbol", |s| s.name = v) />
            <p class="hint">{format!("{} pins, {} graphics. Press P to add pins; numbers increase automatically.", sym.pins.len(), sym.graphics.len())}</p>
        }
        .into_any(),
        _ => view! { <p class="hint">{format!("{} items selected", sel.len())}</p> }.into_any(),
    }
}

fn fp_props(app: &AppState, id: &str, sel: &[Id]) -> impl IntoView {
    use crate::editor::footprint::{edit_footprint, GFX};
    let app = *app;
    let Some(fp) = app
        .project
        .with_untracked(|p| p.library.footprints.get(id).cloned())
    else {
        return view! { <p>"Footprint not found"</p> }.into_any();
    };
    let fid = StoredValue::new(id.to_string());
    match sel {
        [one] if one.0 < GFX => {
            let i = one.0 as usize;
            let Some(pad) = fp.pads.get(i).cloned() else { return view! { <p></p> }.into_any() };
            let shapes = vec![("rect".into(), "Rectangle".into()), ("round".into(), "Round".into()), ("oval".into(), "Oval".into()), ("roundrect".into(), "Rounded rectangle".into())];
            let cur_shape = match pad.shape {
                PadShape::Rect => "rect",
                PadShape::Round => "round",
                PadShape::Oval => "oval",
                PadShape::RoundRect { .. } => "roundrect",
            };
            let kinds = vec![("smd".into(), "SMD".into()), ("tht".into(), "Plated hole".into()), ("npth".into(), "Non-plated hole".into())];
            let cur_kind = match pad.kind {
                PadKind::Smd => "smd",
                PadKind::Tht { .. } => "tht",
                PadKind::Npth { .. } => "npth",
            };
            let drill = pad.drill();
            view! {
                <h4>"Pad"</h4>
                <TextProp label="Number" value=pad.number.clone() on_set=move |v| edit_footprint(&app, &fid.get_value(), "Pad number", |f| if let Some(p) = f.pads.get_mut(i) { p.number = v }) />
                <SelectProp label="Type" options=kinds value=cur_kind.into() on_set=move |v| edit_footprint(&app, &fid.get_value(), "Pad type", |f| if let Some(p) = f.pads.get_mut(i) {
                    let d = p.drill().unwrap_or(p.size.x.min(p.size.y) / 2);
                    p.kind = match v.as_str() { "tht" => PadKind::Tht { drill: d }, "npth" => PadKind::Npth { drill: d }, _ => PadKind::Smd };
                }) />
                <SelectProp label="Shape" options=shapes value=cur_shape.into() on_set=move |v| edit_footprint(&app, &fid.get_value(), "Pad shape", |f| if let Some(p) = f.pads.get_mut(i) {
                    p.shape = match v.as_str() { "round" => PadShape::Round, "oval" => PadShape::Oval, "roundrect" => PadShape::RoundRect { ratio_pct: 25 }, _ => PadShape::Rect };
                }) />
                <LenProp label="Width" nm=pad.size.x on_set=move |v| edit_footprint(&app, &fid.get_value(), "Pad size", |f| if let Some(p) = f.pads.get_mut(i) { p.size.x = v }) />
                <LenProp label="Height" nm=pad.size.y on_set=move |v| edit_footprint(&app, &fid.get_value(), "Pad size", |f| if let Some(p) = f.pads.get_mut(i) { p.size.y = v }) />
                {drill.map(|d| view! { <LenProp label="Drill" nm=d on_set=move |v| edit_footprint(&app, &fid.get_value(), "Drill", |f| if let Some(p) = f.pads.get_mut(i) {
                    match &mut p.kind { PadKind::Tht { drill } | PadKind::Npth { drill } => *drill = v, PadKind::Smd => {} }
                }) /> })}
                <LenProp label="X" nm=pad.pos.x on_set=move |v| edit_footprint(&app, &fid.get_value(), "Pad position", |f| if let Some(p) = f.pads.get_mut(i) { p.pos.x = v }) />
                <LenProp label="Y" nm=pad.pos.y on_set=move |v| edit_footprint(&app, &fid.get_value(), "Pad position", |f| if let Some(p) = f.pads.get_mut(i) { p.pos.y = v }) />
                <NumProp label="Rotation °" value=pad.rotation on_set=move |v| edit_footprint(&app, &fid.get_value(), "Pad rotation", |f| if let Some(p) = f.pads.get_mut(i) { p.rotation = normalize_angle(v) }) />
            }
            .into_any()
        }
        [] => view! {
            <h4>"Footprint"</h4>
            <TextProp label="Name" value=fp.name.clone() on_set=move |v| edit_footprint(&app, &fid.get_value(), "Rename footprint", |f| f.name = v) />
            <p class="hint">{format!("{} pads. Press P for SMD pads, H for through-hole pads; new pads copy the last pad's size.", fp.pads.len())}</p>
            <PadGenerator id=fid.get_value() />
        }
        .into_any(),
        _ => view! { <p class="hint">{format!("{} items selected", sel.len())}</p> }.into_any(),
    }
}

#[component]
fn LayersPanel() -> impl IntoView {
    let app = use_app();
    let layers = Memo::new(move |_| {
        let b = app.tab().board();
        app.project.with(|p| {
            b.and_then(|b| p.board(b))
                .map(|bd| Layer::all_for(bd.pcb.copper_layers))
                .unwrap_or_default()
        })
    });
    view! {
        <div class="panel-title"><Icon name="layers" />"Layers"</div>
        <div class="layers">
            {move || layers.get().into_iter().map(|l| {
                let hidden = move || app.hidden_layers.with(|h| h.contains(&l));
                let active = move || app.active_layer.get() == l;
                view! {
                    <div class="layer" class:active=active class:hidden=hidden>
                        <span class="swatch" style=format!("background: #{:06x}", l.color())></span>
                        <button class="lname" on:click=move |_| if l.is_copper() && l != Layer::Multi { app.active_layer.set(l) } title="Make active">{l.name()}</button>
                        <button class="icon" title="Show / hide" on:click=move |_| app.hidden_layers.update(|h| if let Some(i) = h.iter().position(|x| *x == l) { h.remove(i); } else { h.push(l); })>
                            {move || if hidden() { view! { <Icon name="eye-off" /> } } else { view! { <Icon name="eye" /> } }}
                        </button>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

#[component]
fn NetsPanel() -> impl IntoView {
    let app = use_app();
    let nets = Memo::new(move |_| {
        app.rev.track();
        let b = app.tab().board();
        app.project.with_untracked(|p| {
            b.and_then(|b| p.board(b))
                .map(|bd| bd.pcb.nets())
                .unwrap_or_default()
        })
    });
    view! {
        <div class="panel-title"><Icon name="list" />"Nets"
            <span class="spacer"></span>
            <button class="icon" title="Clear highlight" on:click=move |_| app.highlight_net.set(None)><Icon name="x" /></button>
        </div>
        <div class="nets">
            {move || nets.get().into_iter().map(|n| {
                let n2 = n.clone();
                let n3 = n.clone();
                view! { <button class="net-item" class:active=move || app.highlight_net.get().as_deref() == Some(n2.as_str()) on:click=move |_| app.highlight_net.set(Some(n3.clone()))>{n}</button> }
            }).collect_view()}
        </div>
    }
}

/// Quick pad rows for headers, SOIC, DIP and similar packages.
#[component]
fn PadGenerator(id: String) -> impl IntoView {
    use crate::editor::footprint::edit_footprint;
    use easypro_core::geom::Point;
    let app = use_app();
    let count = RwSignal::new(8usize);
    let rows = RwSignal::new(2usize);
    let pitch = RwSignal::new(easypro_core::units::mm(1.27));
    let span = RwSignal::new(easypro_core::units::mm(5.4));
    let w = RwSignal::new(easypro_core::units::mm(1.55));
    let h = RwSignal::new(easypro_core::units::mm(0.6));
    let tht = RwSignal::new(false);
    let drill = RwSignal::new(easypro_core::units::mm(0.8));
    let id = StoredValue::new(id);
    let len = move |label: &'static str, sig: RwSignal<i64>| view! { <LenProp label=label nm=sig.get_untracked() on_set=move |v| sig.set(v) /> };
    let generate = move |_| {
        let kind = if tht.get_untracked() {
            PadKind::Tht {
                drill: drill.get_untracked(),
            }
        } else {
            PadKind::Smd
        };
        let pads = easypro_core::wizard::pad_array(
            count.get_untracked().max(1),
            rows.get_untracked(),
            pitch.get_untracked(),
            span.get_untracked(),
            Point::new(w.get_untracked(), h.get_untracked()),
            kind,
        );
        let n = pads.len();
        edit_footprint(&app, &id.get_value(), "Generate pads", |f| f.pads = pads);
        app.ok(format!("Generated {n} pads"));
    };
    view! {
        <h4>"Generate Pads"</h4>
        <NumProp label="Pad count" value=count.get_untracked() as f64 on_set=move |v| count.set(v.max(1.0) as usize) />
        <SelectProp label="Rows" options=vec![("1".into(), "Single row".into()), ("2".into(), "Two rows (SOIC/DIP)".into())] value=rows.get_untracked().to_string() on_set=move |v| rows.set(v.parse().unwrap_or(2)) />
        {len("Pitch", pitch)}
        {len("Row spacing", span)}
        {len("Pad width", w)}
        {len("Pad height", h)}
        <CheckProp label="Through-hole" value=tht.get_untracked() on_set=move |v| tht.set(v) />
        {len("Drill", drill)}
        <div class="row"><button on:click=generate><Icon name="wand" />"Replace Pads"</button></div>
    }
}
