//! Device page: one place to tie a symbol, a footprint, a 3D body and the
//! part's parameters together, with live previews and pin mapping.

use crate::actions;
use crate::icons::Icon;
use crate::state::{use_app, AppState, Tab};
use crate::storage;
use easypro_core::geom::Point;
use easypro_core::library::{Body, Device, Model3d};
use easypro_core::scene::{footprint_scene, symbol_scene, SchTheme};
use easypro_core::units::mm;
use easypro_export::svg::scene_to_svg;
use leptos::prelude::*;
use leptos::task::spawn_local;

fn edit_device(app: &AppState, id: &str, label: &str, f: impl FnOnce(&mut Device)) {
    let id = id.to_string();
    app.edit(label, |p| {
        if let Some(d) = p.library.devices.get_mut(&id) {
            f(d);
        }
    });
}

pub fn symbol_svg(app: &AppState, sym: &str) -> String {
    let dark = app.dark.get_untracked();
    app.project.with_untracked(|p| {
        p.library
            .symbols
            .get(sym)
            .or_else(|| easypro_core::builtin::library().symbols.get(sym))
            .map(|s| scene_to_svg(&symbol_scene(s, if dark { &SchTheme::DARK } else { &SchTheme::LIGHT }), None, mm(1.5)))
            .unwrap_or_default()
    })
}

pub fn footprint_svg(app: &AppState, fp: &str) -> String {
    app.project.with_untracked(|p| {
        p.library
            .footprints
            .get(fp)
            .or_else(|| easypro_core::builtin::library().footprints.get(fp))
            .map(|f| scene_to_svg(&footprint_scene(f), Some(0x14161a), mm(1.0)))
            .unwrap_or_default()
    })
}

const BODIES: [(&str, &str); 7] = [
    ("chip", "Chip (R/C)"),
    ("ic", "IC with gull-wing leads"),
    ("dip", "DIP through-hole"),
    ("header", "Pin header"),
    ("cylinder", "Cylinder (electrolytic)"),
    ("led", "LED with dome"),
    ("box", "Box"),
];

fn body_key(b: &Body) -> &'static str {
    match b {
        Body::Chip => "chip",
        Body::Ic => "ic",
        Body::Dip => "dip",
        Body::Header => "header",
        Body::Cylinder => "cylinder",
        Body::Led => "led",
        Body::Box => "box",
        Body::Mesh { .. } => "mesh",
    }
}

fn body_from(k: &str) -> Option<Body> {
    Some(match k {
        "chip" => Body::Chip,
        "ic" => Body::Ic,
        "dip" => Body::Dip,
        "header" => Body::Header,
        "cylinder" => Body::Cylinder,
        "led" => Body::Led,
        "box" => Body::Box,
        _ => return None,
    })
}

#[component]
fn Field(label: &'static str, children: Children) -> impl IntoView {
    view! { <label class="field"><span>{label}</span>{children()}</label> }
}

#[component]
pub fn DevicePage(id: String) -> impl IntoView {
    let app = use_app();
    let dev = {
        let id = id.clone();
        Memo::new(move |_| {
            app.rev.track();
            app.project.with_untracked(|p| p.library.devices.get(&id).cloned())
        })
    };
    let id = StoredValue::new(id);
    let text_field = move |label: &'static str, get: fn(&Device) -> String, set: fn(&mut Device, String)| {
        view! {
            <Field label=label>
                <input type="text" prop:value=move || dev.get().map(|d| get(&d)).unwrap_or_default()
                    on:change=move |ev| { let v = event_target_value(&ev); edit_device(&app, &id.get_value(), label, |d| set(d, v)); } />
            </Field>
        }
    };
    let symbols = move || app.project.with(|p| p.library.symbols.values().map(|s| (s.id.clone(), s.name.clone())).collect::<Vec<_>>());
    let footprints = move || app.project.with(|p| p.library.footprints.values().map(|s| (s.id.clone(), s.name.clone())).collect::<Vec<_>>());
    let mm_input = move |label: &'static str, get: fn(&Model3d) -> i64, set: fn(&mut Model3d, i64)| {
        view! {
            <Field label=label>
                <input type="number" step="0.1" prop:value=move || dev.get().and_then(|d| d.model).map(|m| format!("{}", get(&m) as f64 / 1e6)).unwrap_or_default()
                    on:change=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                            edit_device(&app, &id.get_value(), "Edit 3D body", |d| if let Some(m) = d.model.as_mut() { set(m, mm(v)) });
                        }
                    } />
            </Field>
        }
    };
    let import_obj = move |_| {
        spawn_local(async move {
            let Some((name, text)) = storage::pick_file(".obj").await else { return };
            match easypro_view3d::build::parse_obj(&text) {
                Ok(body) => {
                    edit_device(&app, &id.get_value(), "Import 3D model", |d| {
                        let mut m = d.model.clone().unwrap_or(Model3d { body: Body::Box, size: Point::new(mm(5.0), mm(5.0)), height: mm(2.0), standoff: 0, color: 0x808080, offset: Point::ZERO });
                        m.body = body;
                        d.model = Some(m);
                    });
                    app.ok(format!("Imported 3D model from {name}"));
                }
                Err(e) => app.error(format!("{name}: {e}")),
            }
        });
    };

    view! {
        <div class="page device-page">
            {move || match dev.get() {
                None => view! { <div class="empty">"This device no longer exists."</div> }.into_any(),
                Some(d) => {
                    let sym_id = d.symbol.clone();
                    let fp_id = d.footprint.clone().unwrap_or_default();
                    let pins = app.project.with_untracked(|p| p.library.symbols.get(&sym_id).map(|s| s.pins.iter().map(|x| (x.number.clone(), x.name.clone())).collect::<Vec<_>>()).unwrap_or_default());
                    let pads: Vec<String> = app.project.with_untracked(|p| {
                        let mut v: Vec<String> = p.library.footprints.get(&fp_id).map(|f| f.pads.iter().map(|x| x.number.clone()).collect()).unwrap_or_default();
                        v.dedup();
                        v
                    });
                    let model_key = d.model.as_ref().map(|m| body_key(&m.body));
                    let has_model = model_key.is_some();
                    let attrs: Vec<(String, String)> = d.attributes.clone().into_iter().collect();
                    let sym_open = sym_id.clone();
                    let fp_open = fp_id.clone();
                    let place_id = d.id.clone();
                    view! {
                        <div class="device-head">
                            <h2><Icon name="chip" />{d.name.clone()}</h2>
                            <div class="actions">
                                <button class="primary" on:click=move |_| actions::place_device(&app, &place_id)><Icon name="plus" />"Place in Schematic"</button>
                            </div>
                        </div>
                        <div class="device-grid">
                            <section class="card">
                                <h3>"Part"</h3>
                                {text_field("Name", |d| d.name.clone(), |d, v| d.name = v)}
                                {text_field("Default value", |d| d.value.clone(), |d, v| d.value = v)}
                                {text_field("Designator prefix", |d| d.prefix.clone(), |d, v| d.prefix = v)}
                                {text_field("Category", |d| d.category.clone(), |d, v| d.category = v)}
                                {text_field("Description", |d| d.description.clone(), |d, v| d.description = v)}
                                <h3>"Attributes"</h3>
                                <table class="kv">
                                    <tbody>
                                    {attrs.into_iter().map(|(k, v)| {
                                        let k2 = k.clone();
                                        let k3 = k.clone();
                                        view! {
                                            <tr>
                                                <td>{k.clone()}</td>
                                                <td><input type="text" prop:value=v on:change=move |ev| { let v = event_target_value(&ev); let k = k2.clone(); edit_device(&app, &id.get_value(), "Edit attribute", |d| { d.attributes.insert(k, v); }); } /></td>
                                                <td><button class="icon" title="Remove" on:click=move |_| { let k = k3.clone(); edit_device(&app, &id.get_value(), "Remove attribute", |d| { d.attributes.remove(&k); }); }><Icon name="x" /></button></td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                    </tbody>
                                </table>
                                <div class="row">
                                    {["Manufacturer", "MPN", "LCSC", "Datasheet"].into_iter().map(|k| view! {
                                        <button class="chip-btn" on:click=move |_| edit_device(&app, &id.get_value(), "Add attribute", |d| { d.attributes.entry(k.to_string()).or_default(); })>"+ "{k}</button>
                                    }).collect_view()}
                                </div>
                            </section>
                            <section class="card">
                                <h3>"Symbol"</h3>
                                <select on:change=move |ev| { let v = event_target_value(&ev); edit_device(&app, &id.get_value(), "Change symbol", |d| d.symbol = v); }>
                                    {symbols().into_iter().map(|(sid, name)| { let sel = sid == sym_id; view! { <option value=sid selected=sel>{name}</option> } }).collect_view()}
                                </select>
                                <div class="preview light" inner_html=symbol_svg(&app, &d.symbol)></div>
                                <button on:click=move |_| app.open(Tab::Symbol { id: sym_open.clone() })><Icon name="schematic" />"Edit Symbol"</button>
                            </section>
                            <section class="card">
                                <h3>"Footprint"</h3>
                                <select on:change=move |ev| { let v = event_target_value(&ev); edit_device(&app, &id.get_value(), "Change footprint", |d| d.footprint = (!v.is_empty()).then_some(v)); }>
                                    <option value="">"(none — schematic only)"</option>
                                    {footprints().into_iter().map(|(fid, name)| { let sel = Some(&fid) == d.footprint.as_ref(); view! { <option value=fid selected=sel>{name}</option> } }).collect_view()}
                                </select>
                                <div class="preview dark" inner_html=footprint_svg(&app, &fp_id)></div>
                                <button disabled=fp_id.is_empty() on:click=move |_| app.open(Tab::Footprint { id: fp_open.clone() })><Icon name="footprint" />"Edit Footprint"</button>
                            </section>
                            <section class="card">
                                <h3>"Pin Mapping"</h3>
                                <p class="hint">"Each symbol pin connects to the footprint pad with the same number unless you choose another pad here."</p>
                                <table class="kv pinmap">
                                    <thead><tr><th>"Pin"</th><th>"Name"</th><th>"Pad"</th></tr></thead>
                                    <tbody>
                                    {pins.into_iter().map(|(num, name)| {
                                        let current = d.pad_for_pin(&num).to_string();
                                        let missing = !pads.contains(&current);
                                        let num2 = num.clone();
                                        view! {
                                            <tr class:bad=missing>
                                                <td>{num.clone()}</td>
                                                <td>{name}</td>
                                                <td>
                                                    <select on:change=move |ev| {
                                                        let v = event_target_value(&ev);
                                                        let n = num2.clone();
                                                        edit_device(&app, &id.get_value(), "Map pin", |d| { if v == n { d.pin_map.remove(&n); } else { d.pin_map.insert(n, v); } });
                                                    }>
                                                        {missing.then(|| view! { <option selected=true>{format!("{current} (missing)")}</option> })}
                                                        {pads.iter().map(|p| { let sel = *p == current; view! { <option value=p.clone() selected=sel>{p.clone()}</option> } }).collect_view()}
                                                    </select>
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                    </tbody>
                                </table>
                            </section>
                            <section class="card">
                                <h3>"3D Model"</h3>
                                <Field label="Body">
                                    <select on:change=move |ev| {
                                        let v = event_target_value(&ev);
                                        edit_device(&app, &id.get_value(), "Change 3D body", |d| {
                                            match body_from(&v) {
                                                Some(b) => {
                                                    let mut m = d.model.clone().unwrap_or(Model3d { body: Body::Box, size: Point::new(mm(3.0), mm(3.0)), height: mm(1.5), standoff: 0, color: 0x202020, offset: Point::ZERO });
                                                    m.body = b;
                                                    d.model = Some(m);
                                                }
                                                None => if v == "none" { d.model = None },
                                            }
                                        });
                                    }>
                                        <option value="none" selected=!has_model>"None"</option>
                                        {BODIES.iter().map(|(k, label)| { let sel = model_key == Some(*k); view! { <option value=*k selected=sel>{*label}</option> } }).collect_view()}
                                        {(model_key == Some("mesh")).then(|| view! { <option value="mesh" selected=true>"Imported mesh"</option> })}
                                    </select>
                                </Field>
                                {has_model.then(|| view! {
                                    <div class="grid2">
                                        {mm_input("Length (mm)", |m| m.size.x, |m, v| m.size.x = v)}
                                        {mm_input("Width (mm)", |m| m.size.y, |m, v| m.size.y = v)}
                                        {mm_input("Height (mm)", |m| m.height, |m, v| m.height = v)}
                                        {mm_input("Standoff (mm)", |m| m.standoff, |m, v| m.standoff = v)}
                                    </div>
                                    <Field label="Colour">
                                        <input type="color" prop:value=move || dev.get().and_then(|d| d.model).map(|m| format!("#{:06x}", m.color)).unwrap_or_default()
                                            on:change=move |ev| {
                                                let v = event_target_value(&ev);
                                                if let Ok(c) = u32::from_str_radix(v.trim_start_matches('#'), 16) {
                                                    edit_device(&app, &id.get_value(), "3D colour", |d| if let Some(m) = d.model.as_mut() { m.color = c });
                                                }
                                            } />
                                    </Field>
                                })}
                                <button on:click=import_obj><Icon name="import" />"Import OBJ Mesh…"</button>
                                <p class="hint">"Leads and pins are generated from the footprint pads, so the body always lines up with the copper."</p>
                            </section>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
