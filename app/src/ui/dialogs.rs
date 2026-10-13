//! Modal dialogs.

use crate::actions;
use crate::commands::{self, COMMANDS};
use crate::icons::Icon;
use crate::js;
use crate::state::{use_app, AppState, Dialog, Tab, Theme};
use crate::storage;
use easypro_core::library::{Device, PinKind};
use easypro_core::pcb::NetClass;
use easypro_core::units::{mil, mm, parse_length, Unit};
use easypro_export::fab::{package_files, package_zip, Fab};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
fn Modal(title: String, #[prop(optional)] wide: bool, children: Children) -> impl IntoView {
    let app = use_app();
    view! {
        <div class="overlay" on:click=move |_| app.dialog.set(None)>
            <div class="modal" class:wide=wide role="dialog" aria-modal="true" on:click=|ev| ev.stop_propagation()>
                <div class="modal-head">
                    <h3>{title}</h3>
                    <button class="icon" title="Close (Esc)" on:click=move |_| app.dialog.set(None)><Icon name="x" /></button>
                </div>
                <div class="modal-body">{children()}</div>
            </div>
        </div>
    }
}

#[component]
pub fn Dialogs() -> impl IntoView {
    let app = use_app();
    move || {
        app.dialog.get().map(|d| match d {
            Dialog::NewProject => view! { <NewProject /> }.into_any(),
            Dialog::Rename => view! { <Rename /> }.into_any(),
            Dialog::Settings => view! { <Settings /> }.into_any(),
            Dialog::Shortcuts => view! { <Shortcuts /> }.into_any(),
            Dialog::About => view! { <About /> }.into_any(),
            Dialog::Fabrication => view! { <Fabrication /> }.into_any(),
            Dialog::Rules => view! { <Rules /> }.into_any(),
            Dialog::BoardSetup => view! { <BoardSetup /> }.into_any(),
            Dialog::History => view! { <History /> }.into_any(),
            Dialog::IcWizard => view! { <IcWizard /> }.into_any(),
        })
    }
}

#[component]
fn NewProject() -> impl IntoView {
    let app = use_app();
    let name = RwSignal::new("My Board".to_string());
    let go = move || {
        actions::create_project(&app, &name.get_untracked());
        app.dialog.set(None);
    };
    view! {
        <Modal title="New Project".into()>
            <label class="field"><span>"Project name"</span>
                <input type="text" autofocus prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev))
                    on:keydown=move |ev: web_sys::KeyboardEvent| if ev.key() == "Enter" { go() } />
            </label>
            <p class="hint">"A new project has one schematic sheet and a 50 × 40 mm two-layer board. The current project stays in this browser's project list."</p>
            <div class="modal-foot"><button on:click=move |_| app.dialog.set(None)>"Cancel"</button><button class="primary" on:click=move |_| go()>"Create"</button></div>
        </Modal>
    }
}

#[component]
fn Rename() -> impl IntoView {
    let app = use_app();
    let name = RwSignal::new(app.project.with_untracked(|p| p.name.clone()));
    let go = move || {
        let n = name.get_untracked().trim().to_string();
        if !n.is_empty() {
            app.edit("Rename project", |p| p.name = n);
            actions::persist(&app);
        }
        app.dialog.set(None);
    };
    view! {
        <Modal title="Rename Project".into()>
            <label class="field"><span>"Name"</span>
                <input type="text" autofocus prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev))
                    on:keydown=move |ev: web_sys::KeyboardEvent| if ev.key() == "Enter" { go() } />
            </label>
            <div class="modal-foot"><button on:click=move |_| app.dialog.set(None)>"Cancel"</button><button class="primary" on:click=move |_| go()>"Rename"</button></div>
        </Modal>
    }
}

fn set_settings(app: &AppState, f: impl FnOnce(&mut crate::state::Settings)) {
    app.settings.update(f);
    app.settings.with_untracked(|s| s.save());
}

#[component]
fn Settings() -> impl IntoView {
    let app = use_app();
    let s = app.settings.get_untracked();
    view! {
        <Modal title="Preferences".into()>
            <div class="form">
                <label class="field"><span>"Theme"</span>
                    <select on:change=move |ev| { let v = event_target_value(&ev); set_settings(&app, |s| s.theme = match v.as_str() { "light" => Theme::Light, "dark" => Theme::Dark, _ => Theme::System }); }>
                        <option value="system" selected=s.theme == Theme::System>"Follow system"</option>
                        <option value="light" selected=s.theme == Theme::Light>"Light"</option>
                        <option value="dark" selected=s.theme == Theme::Dark>"Dark"</option>
                    </select>
                </label>
                <label class="field"><span>"Units"</span>
                    <select on:change=move |ev| { let v = event_target_value(&ev); set_settings(&app, |s| s.unit = match v.as_str() { "mil" => Unit::Mil, "inch" => Unit::Inch, _ => Unit::Mm }); }>
                        <option value="mm" selected=s.unit == Unit::Mm>"Millimetres"</option>
                        <option value="mil" selected=s.unit == Unit::Mil>"Mils"</option>
                        <option value="inch" selected=s.unit == Unit::Inch>"Inches"</option>
                    </select>
                </label>
                <label class="field"><span>"Schematic grid"</span>
                    <select on:change=move |ev| { let v: f64 = event_target_value(&ev).parse().unwrap_or(50.0); set_settings(&app, |s| s.sch_grid = mil(v)); }>
                        {[10.0, 25.0, 50.0, 100.0].into_iter().map(|g| view! { <option value=g.to_string() selected=s.sch_grid == mil(g)>{format!("{g} mil")}</option> }).collect_view()}
                    </select>
                </label>
                <label class="field"><span>"PCB grid"</span>
                    <input type="text" prop:value=Unit::Mm.format(s.pcb_grid) on:change=move |ev| {
                        if let Some(v) = parse_length(&event_target_value(&ev), Unit::Mm).filter(|v| *v > 0) { set_settings(&app, |s| s.pcb_grid = v); }
                    } />
                    <em>"mm (or e.g. 5mil)"</em>
                </label>
                <label class="field check"><input type="checkbox" prop:checked=s.grid_lines on:change=move |ev| { let v = event_target_checked(&ev); set_settings(&app, |s| s.grid_lines = v); } /><span>"Line grid instead of dots"</span></label>
                <label class="field check"><input type="checkbox" prop:checked=s.snap_to_pins on:change=move |ev| { let v = event_target_checked(&ev); set_settings(&app, |s| s.snap_to_pins = v); } /><span>"Snap wires to nearby pins"</span></label>
                <label class="field check"><input type="checkbox" prop:checked=s.autosave on:change=move |ev| { let v = event_target_checked(&ev); set_settings(&app, |s| s.autosave = v); } /><span>"Save to this browser automatically"</span></label>
                <label class="field check"><input type="checkbox" prop:checked=s.realtime_drc on:change=move |ev| { let v = event_target_checked(&ev); set_settings(&app, |s| s.realtime_drc = v); } /><span>"Check design rules while editing the PCB"</span></label>
            </div>
            <div class="modal-foot">
                <button on:click=move |_| { set_settings(&app, |s| *s = Default::default()); app.dialog.set(None); }>"Reset to defaults"</button>
                <button class="primary" on:click=move |_| app.dialog.set(None)>"Done"</button>
            </div>
        </Modal>
    }
}

#[component]
fn Shortcuts() -> impl IntoView {
    let app = use_app();
    let capture = RwSignal::new(None::<&'static str>);
    let filter = RwSignal::new(String::new());
    // While capturing, the next chord becomes the binding.
    let handle = window_event_listener(leptos::ev::keydown, move |ev| {
        if let Some(id) = capture.get_untracked() {
            let ch = commands::chord(&ev);
            if ch.is_empty() {
                return;
            }
            ev.prevent_default();
            ev.stop_propagation();
            if ch != "Escape" {
                set_settings(&app, |s| {
                    s.hotkeys.insert(id.to_string(), ch);
                });
            }
            capture.set(None);
        }
    });
    on_cleanup(move || handle.remove());
    view! {
        <Modal title="Keyboard Shortcuts".into() wide=true>
            <input class="filter" type="search" placeholder="Filter commands" prop:value=move || filter.get() on:input=move |ev| filter.set(event_target_value(&ev)) />
            <p class="hint">"Click a shortcut to change it, then press the new keys (Esc cancels). Your choices are saved in this browser."</p>
            <div class="shortcut-table">
                {move || {
                    app.settings.track();
                    let f = filter.get().to_lowercase();
                    COMMANDS.iter().filter(|c| !c.menu.is_empty()).filter(|c| f.is_empty() || c.label.to_lowercase().contains(&f) || c.menu.to_lowercase().contains(&f)).map(|c| {
                        let id = c.id;
                        let key = commands::key_for(&app, c);
                        let custom = app.settings.with_untracked(|s| s.hotkeys.contains_key(id));
                        let conflicts = !key.is_empty() && COMMANDS.iter().any(|o| o.id != id && !o.menu.is_empty() && o.ctx == c.ctx && commands::key_for(&app, o).eq_ignore_ascii_case(&key));
                        view! {
                            <div class="sc-row" class:conflict=conflicts>
                                <span class="sc-menu">{c.menu}</span>
                                <span class="sc-label">{c.label}</span>
                                <button class="sc-key" on:click=move |_| capture.set(Some(id))>
                                    {move || if capture.get() == Some(id) { "Press keys…".to_string() } else if key.is_empty() { "—".to_string() } else { key.clone() }}
                                </button>
                                {custom.then(|| view! { <button class="icon" title="Reset" on:click=move |_| set_settings(&app, |s| { s.hotkeys.remove(id); })><Icon name="undo" /></button> })}
                            </div>
                        }
                    }).collect_view()
                }}
            </div>
            <div class="modal-foot">
                <button on:click=move |_| set_settings(&app, |s| s.hotkeys.clear())>"Reset all"</button>
                <button class="primary" on:click=move |_| app.dialog.set(None)>"Done"</button>
            </div>
        </Modal>
    }
}

#[component]
fn About() -> impl IntoView {
    view! {
        <Modal title="About EasyPRO".into()>
            <p><b>{concat!("EasyPRO ", env!("CARGO_PKG_VERSION"))}</b>" — schematic capture, PCB layout and 3D preview in the browser."</p>
            <p>"Free and open source under the MIT licence. Written in Rust and compiled to WebAssembly; the editors draw with WebGL2."</p>
            <ul class="plain">
                <li>"Local-first: projects stay in this browser and in files you save. No account, no upload, works offline once loaded."</li>
                <li>"Open format: readable JSON that diffs well in git."</li>
                <li>"Fab-agnostic exports: Gerber X2, Excellon, BOM and pick-and-place for JLCPCB, PCBWay, OSH Park, Eurocircuits or anyone."</li>
                <li>"Imports KiCad footprints and symbols; exports KiCad netlists."</li>
            </ul>
            <p><a href="https://github.com/Flotrpy/EDA-EasyPRO" target="_blank" rel="noopener">"Source code on GitHub"</a></p>
        </Modal>
    }
}

#[component]
fn Fabrication() -> impl IntoView {
    let app = use_app();
    let fab = RwSignal::new(match app.settings.with_untracked(|s| s.fab.clone()).as_str() {
        "pcbway" => Fab::Pcbway,
        "oshpark" => Fab::Oshpark,
        "eurocircuits" => Fab::Eurocircuits,
        "generic" => Fab::Generic,
        _ => Fab::Jlcpcb,
    });
    let board = actions::board_id(&app);
    let base = app.project.with_untracked(|p| p.name.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect::<String>());
    let base = StoredValue::new(if base.is_empty() { "board".to_string() } else { base });
    let check = app.project.with_untracked(|p| {
        board.and_then(|b| p.board(b)).map(|bd| easypro_core::drc::run(&bd.pcb, &p.library, app.unit()))
    }).unwrap_or_default();
    let errors = check.iter().filter(|i| i.severity == easypro_core::check::Severity::Error).count();
    let unrouted = check.iter().filter(|i| i.message.contains("unrouted")).count();
    let check_class = if errors > 0 { "check bad" } else { "check good" };
    let files = Memo::new(move |_| {
        let f = fab.get();
        app.project.with_untracked(|p| board.and_then(|b| p.board(b)).map(|bd| package_files(bd, &p.library, f, &base.get_value()).into_iter().map(|(n, d)| (n, d.len())).collect::<Vec<_>>())).unwrap_or_default()
    });
    let download = move |_| {
        let f = fab.get_untracked();
        let key = match f {
            Fab::Jlcpcb => "jlcpcb",
            Fab::Pcbway => "pcbway",
            Fab::Oshpark => "oshpark",
            Fab::Eurocircuits => "eurocircuits",
            Fab::Generic => "generic",
        };
        set_settings(&app, |s| s.fab = key.into());
        let bytes = app.project.with_untracked(|p| board.and_then(|b| p.board(b)).map(|bd| package_zip(bd, &p.library, f, &base.get_value())));
        if let Some(bytes) = bytes {
            let name = format!("{}-{}-fab.zip", base.get_value(), key);
            js::download(&name, &bytes, "application/zip");
            app.ok(format!("Downloaded {name}"));
        }
    };
    view! {
        <Modal title="Fabrication Package".into() wide=true>
            <div class="fab">
                <div class="fab-left">
                    <h4>"Manufacturer"</h4>
                    {Fab::ALL.into_iter().map(|f| view! {
                        <label class="radio">
                            <input type="radio" name="fab" prop:checked=move || fab.get() == f on:change=move |_| fab.set(f) />
                            <span><b>{f.label()}</b><br /><small>{f.notes()}</small></span>
                        </label>
                    }).collect_view()}
                    <div class=check_class>
                        {if errors > 0 {
                            format!("DRC found {errors} error(s){}. You can still export, but check the Issues panel first.", if unrouted > 0 { " and unrouted nets" } else { "" })
                        } else if unrouted > 0 {
                            "No rule errors, but some nets are not routed yet.".to_string()
                        } else {
                            "DRC passed.".to_string()
                        }}
                    </div>
                </div>
                <div class="fab-right">
                    <h4>"Files"</h4>
                    <table class="kv small">
                        <tbody>
                        {move || files.get().into_iter().map(|(n, size)| view! { <tr><td>{n}</td><td class="num">{format!("{:.1} KB", size as f64 / 1024.0)}</td></tr> }).collect_view()}
                        </tbody>
                    </table>
                    <p class="hint">"The zip is built here in your browser. Upload it to the manufacturer's order page yourself; EasyPRO never contacts them."</p>
                </div>
            </div>
            <div class="modal-foot">
                <button on:click=move |_| { app.dialog.set(None); actions::drc(&app); }>"Show DRC Issues"</button>
                <button class="primary" on:click=download><Icon name="download" />"Download ZIP"</button>
            </div>
        </Modal>
    }
}

#[component]
fn RuleField(label: &'static str, nm: i64, on_set: impl Fn(i64) + 'static) -> impl IntoView {
    let app = use_app();
    let unit = app.unit();
    view! {
        <label class="field">
            <span>{label}</span>
            <input type="text" prop:value=unit.format(nm) on:change=move |ev| if let Some(v) = parse_length(&event_target_value(&ev), unit).filter(|v| *v >= 0) { on_set(v) } />
            <em>{unit.label()}</em>
        </label>
    }
}

fn edit_rules(app: &AppState, f: impl FnOnce(&mut easypro_core::pcb::DesignRules)) {
    let Some(b) = actions::board_id(app) else { return };
    app.edit("Design rules", |p| {
        if let Some(bd) = p.board_mut(b) {
            f(&mut bd.pcb.rules);
        }
    });
}

#[component]
fn Rules() -> impl IntoView {
    let app = use_app();
    let rules = Memo::new(move |_| {
        app.rev.track();
        let b = actions::board_id(&app);
        app.project.with_untracked(|p| b.and_then(|b| p.board(b)).map(|bd| bd.pcb.rules.clone())).unwrap_or_default()
    });
    view! {
        <Modal title="Design Rules".into() wide=true>
            {move || {
                let r = rules.get();
                view! {
                    <div class="rules-grid">
                        <RuleField label="Clearance" nm=r.clearance on_set=move |v| edit_rules(&app, |r| r.clearance = v) />
                        <RuleField label="Default track width" nm=r.track_width on_set=move |v| edit_rules(&app, |r| r.track_width = v) />
                        <RuleField label="Minimum track width" nm=r.min_track_width on_set=move |v| edit_rules(&app, |r| r.min_track_width = v) />
                        <RuleField label="Via diameter" nm=r.via_diameter on_set=move |v| edit_rules(&app, |r| r.via_diameter = v) />
                        <RuleField label="Via drill" nm=r.via_drill on_set=move |v| edit_rules(&app, |r| r.via_drill = v) />
                        <RuleField label="Minimum drill" nm=r.min_drill on_set=move |v| edit_rules(&app, |r| r.min_drill = v) />
                        <RuleField label="Minimum annular ring" nm=r.min_annular_ring on_set=move |v| edit_rules(&app, |r| r.min_annular_ring = v) />
                        <RuleField label="Copper to board edge" nm=r.edge_clearance on_set=move |v| edit_rules(&app, |r| r.edge_clearance = v) />
                        <RuleField label="Solder mask expansion" nm=r.mask_expansion on_set=move |v| edit_rules(&app, |r| r.mask_expansion = v) />
                        <RuleField label="Silkscreen line width" nm=r.silk_width on_set=move |v| edit_rules(&app, |r| r.silk_width = v) />
                    </div>
                    <div class="row">
                        <span class="hint">"Presets:"</span>
                        <button class="chip-btn" on:click=move |_| edit_rules(&app, |r| { r.clearance = mm(0.127); r.min_track_width = mm(0.127); r.min_drill = mm(0.3); r.min_annular_ring = mm(0.13); })>"Low-cost fab (5 mil)"</button>
                        <button class="chip-btn" on:click=move |_| edit_rules(&app, |r| { r.clearance = mm(0.2); r.min_track_width = mm(0.15); r.min_drill = mm(0.3); r.min_annular_ring = mm(0.15); })>"Standard (6/8 mil)"</button>
                        <button class="chip-btn" on:click=move |_| edit_rules(&app, |r| { r.clearance = mm(0.3); r.min_track_width = mm(0.3); r.min_drill = mm(0.6); r.min_annular_ring = mm(0.25); })>"Home etching"</button>
                    </div>
                    <h4>"Net classes"</h4>
                    <table class="kv netclasses">
                        <thead><tr><th>"Name"</th><th>"Clearance"</th><th>"Track"</th><th>"Via"</th><th>"Drill"</th><th>"Nets (comma separated)"</th><th></th></tr></thead>
                        <tbody>
                        {r.net_classes.iter().enumerate().map(|(i, c)| {
                            let u = app.unit();
                            let len = move |nm: i64, set: fn(&mut NetClass, i64)| view! {
                                <td><input type="text" prop:value=u.format(nm) on:change=move |ev| if let Some(v) = parse_length(&event_target_value(&ev), u) { edit_rules(&app, |r| if let Some(c) = r.net_classes.get_mut(i) { set(c, v) }) } /></td>
                            };
                            view! {
                                <tr>
                                    <td><input type="text" prop:value=c.name.clone() on:change=move |ev| { let v = event_target_value(&ev); edit_rules(&app, |r| if let Some(c) = r.net_classes.get_mut(i) { c.name = v }) } /></td>
                                    {len(c.clearance, |c, v| c.clearance = v)}
                                    {len(c.track_width, |c, v| c.track_width = v)}
                                    {len(c.via_diameter, |c, v| c.via_diameter = v)}
                                    {len(c.via_drill, |c, v| c.via_drill = v)}
                                    <td><input type="text" prop:value=c.nets.join(", ") on:change=move |ev| { let v = event_target_value(&ev); edit_rules(&app, |r| if let Some(c) = r.net_classes.get_mut(i) { c.nets = v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect() }) } /></td>
                                    <td><button class="icon" title="Remove" on:click=move |_| edit_rules(&app, |r| { r.net_classes.remove(i); })><Icon name="trash" /></button></td>
                                </tr>
                            }
                        }).collect_view()}
                        </tbody>
                    </table>
                    <button on:click=move |_| edit_rules(&app, |r| r.net_classes.push(NetClass { name: "Power".into(), clearance: r.clearance, track_width: mm(0.5), via_diameter: mm(0.8), via_drill: mm(0.4), nets: vec!["VCC".into(), "GND".into()] }))><Icon name="plus" />"Add net class"</button>
                }
            }}
            <div class="modal-foot"><button class="primary" on:click=move |_| { app.dialog.set(None); actions::drc(&app); }>"Done and Run DRC"</button></div>
        </Modal>
    }
}

#[component]
fn BoardSetup() -> impl IntoView {
    let app = use_app();
    let b = actions::board_id(&app);
    let pcb = app.project.with_untracked(|p| b.and_then(|b| p.board(b)).map(|bd| bd.pcb.clone())).unwrap_or_default();
    let edit = move |label: &str, f: Box<dyn FnOnce(&mut easypro_core::pcb::Pcb)>| {
        if let Some(b) = b {
            app.edit(label, |p| {
                let lib = p.library.clone();
                if let Some(bd) = p.board_mut(b) {
                    f(&mut bd.pcb);
                    bd.pcb.rebuild_pours(&lib);
                }
            });
        }
    };
    let masks = [("Green", 0x1f6b3au32), ("Black", 0x15171a), ("Blue", 0x1d3f8f), ("Red", 0x9a1c1c), ("White", 0xe8e8e8), ("Purple", 0x4b2a7a), ("Yellow", 0xc8a21a)];
    view! {
        <Modal title="Board Setup".into()>
            <div class="form">
                <label class="field"><span>"Copper layers"</span>
                    <select on:change=move |ev| { let n: u8 = event_target_value(&ev).parse().unwrap_or(2); edit("Copper layers", Box::new(move |p| p.copper_layers = n)); }>
                        {[1u8, 2, 4, 6].into_iter().map(|n| view! { <option value=n.to_string() selected=pcb.copper_layers == n>{n}</option> }).collect_view()}
                    </select>
                </label>
                <label class="field"><span>"Thickness"</span>
                    <select on:change=move |ev| { let t: f64 = event_target_value(&ev).parse().unwrap_or(1.6); edit("Board thickness", Box::new(move |p| p.thickness = mm(t))); }>
                        {[0.6, 0.8, 1.0, 1.2, 1.6, 2.0].into_iter().map(|t| view! { <option value=t.to_string() selected=pcb.thickness == mm(t)>{format!("{t} mm")}</option> }).collect_view()}
                    </select>
                </label>
                <div class="field"><span>"Solder mask"</span>
                    <div class="row">
                        {masks.into_iter().map(|(name, c)| view! {
                            <button class="mask-swatch" class:on=pcb.mask_color == c title=name style=format!("background: #{c:06x}") on:click=move |_| edit("Mask colour", Box::new(move |p| p.mask_color = c))></button>
                        }).collect_view()}
                    </div>
                </div>
            </div>
            <p class="hint">"Layer count and thickness go into the fab notes; the mask colour shows in the 3D view."</p>
            <div class="modal-foot"><button class="primary" on:click=move |_| app.dialog.set(None)>"Done"</button></div>
        </Modal>
    }
}

#[component]
fn History() -> impl IntoView {
    let app = use_app();
    let list = RwSignal::new(Vec::<(String, storage::Snapshot)>::new());
    let label = RwSignal::new(String::new());
    let refresh = move || {
        let key = app.key.get_untracked();
        spawn_local(async move { list.set(storage::snapshots(&key).await) });
    };
    refresh();
    let take = move |_| {
        let key = app.key.get_untracked();
        let p = app.project.get_untracked();
        let l = label.get_untracked();
        let l = if l.trim().is_empty() { "Snapshot".to_string() } else { l };
        spawn_local(async move {
            let _ = storage::snapshot(&key, &p, &l).await;
            label.set(String::new());
            list.set(storage::snapshots(&key).await);
        });
    };
    view! {
        <Modal title="Version History".into() wide=true>
            <div class="row">
                <input type="text" placeholder="Name this version (optional)" prop:value=move || label.get() on:input=move |ev| label.set(event_target_value(&ev)) />
                <button class="primary" on:click=take><Icon name="history" />"Save Version Now"</button>
            </div>
            <p class="hint">"Versions are stored in this browser. Restoring is undoable with Ctrl+Z."</p>
            <div class="history-list">
                {move || {
                    let l = list.get();
                    if l.is_empty() { return view! { <div class="tree-empty">"No versions yet. Every Ctrl+S also saves one."</div> }.into_any(); }
                    l.into_iter().map(|(id, s)| {
                        let id2 = id.clone();
                        let json = s.project.clone();
                        let when = js::local_time(&s.time);
                        view! {
                            <div class="history-item">
                                <span class="hl">{s.label.clone()}</span>
                                <span class="ht">{when}</span>
                                <button on:click=move |_| {
                                    match easypro_core::project::Project::from_json(&json) {
                                        Ok(p) => { app.edit("Restore version", |cur| *cur = p); app.selection.set(vec![]); app.ok("Version restored"); app.dialog.set(None); }
                                        Err(e) => app.error(e.to_string()),
                                    }
                                }>"Restore"</button>
                                <button class="icon" title="Delete" on:click=move |_| { let id = id2.clone(); spawn_local(async move { let _ = storage::delete_snapshot(&id).await; refresh(); }); }><Icon name="trash" /></button>
                            </div>
                        }
                    }).collect_view().into_any()
                }}
            </div>
        </Modal>
    }
}

fn parse_kind(s: &str) -> PinKind {
    match s.to_lowercase().as_str() {
        "in" | "input" | "i" => PinKind::Input,
        "out" | "output" | "o" => PinKind::Output,
        "io" | "bidir" | "bidirectional" | "b" => PinKind::Bidirectional,
        "pwr" | "power" | "power_in" | "p" => PinKind::PowerIn,
        "power_out" | "pout" => PinKind::PowerOut,
        "oc" | "open_collector" => PinKind::OpenCollector,
        "nc" => PinKind::NotConnected,
        _ => PinKind::Passive,
    }
}

/// Parse "number name [type]" lines (tab, comma or space separated).
pub fn parse_pin_list(text: &str) -> Vec<(String, String, PinKind)> {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let parts: Vec<&str> = l.split(|c: char| c == '\t' || c == ',' || c == ';' || c.is_whitespace()).filter(|s| !s.is_empty()).collect();
            match parts.as_slice() {
                [num] => Some((num.to_string(), num.to_string(), PinKind::Passive)),
                [num, name] => {
                    let kind = if name.to_uppercase().contains("GND") || name.to_uppercase().starts_with("VCC") || name.to_uppercase().starts_with("VDD") { PinKind::PowerIn } else { PinKind::Bidirectional };
                    Some((num.to_string(), name.to_string(), kind))
                }
                [num, name, kind, ..] => Some((num.to_string(), name.to_string(), parse_kind(kind))),
                [] => None,
            }
        })
        .collect()
}

#[component]
fn IcWizard() -> impl IntoView {
    let app = use_app();
    let name = RwSignal::new("MyChip".to_string());
    let prefix = RwSignal::new("U".to_string());
    let footprint = RwSignal::new("fp:SOIC-8".to_string());
    let pins = RwSignal::new("1 VCC power\n2 SDA io\n3 SCL in\n4 GND power\n5 INT out\n6 NC nc\n7 EN in\n8 VOUT power_out".to_string());
    let fps: Vec<(String, String)> = {
        let mut v: Vec<(String, String)> = easypro_core::builtin::library().footprints.values().map(|f| (f.id.clone(), f.name.clone())).collect();
        v.extend(app.project.with_untracked(|p| p.library.footprints.values().map(|f| (f.id.clone(), f.name.clone())).collect::<Vec<_>>()));
        v.sort_by(|a, b| a.1.cmp(&b.1));
        v.dedup();
        v
    };
    let parsed = Memo::new(move |_| parse_pin_list(&pins.get()));
    let preview = Memo::new(move |_| {
        let list = parsed.get();
        let half = list.len().div_ceil(2);
        let refs: Vec<(&str, &str, PinKind)> = list.iter().map(|(a, b, k)| (a.as_str(), b.as_str(), *k)).collect();
        let sym = easypro_core::builtin::symbols::ic_box("sym:preview", &name.get(), &refs[..half], &refs[half..]);
        easypro_export::svg::scene_to_svg(&easypro_core::scene::symbol_scene(&sym, &easypro_core::scene::SchTheme::LIGHT), None, mm(1.5))
    });
    let create = move |_| {
        let list = parsed.get_untracked();
        if list.is_empty() {
            app.warn("Enter at least one pin");
            return;
        }
        let nm = name.get_untracked();
        let slug: String = nm.chars().map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
        let fp = footprint.get_untracked();
        let mut dev_id = String::new();
        app.edit("IC wizard", |p| {
            let sid = (1..).map(|n| if n == 1 { format!("sym:{slug}") } else { format!("sym:{slug}-{n}") }).find(|i| !p.library.symbols.contains_key(i)).unwrap();
            dev_id = (1..).map(|n| if n == 1 { format!("dev:{slug}") } else { format!("dev:{slug}-{n}") }).find(|i| !p.library.devices.contains_key(i)).unwrap();
            let half = list.len().div_ceil(2);
            let refs: Vec<(&str, &str, PinKind)> = list.iter().map(|(a, b, k)| (a.as_str(), b.as_str(), *k)).collect();
            let sym = easypro_core::builtin::symbols::ic_box(&sid, &nm, &refs[..half], &refs[half..]);
            p.library.add_symbol(sym);
            if let Some(f) = easypro_core::builtin::library().footprints.get(&fp) {
                p.library.footprints.entry(f.id.clone()).or_insert_with(|| f.clone());
            }
            p.library.add_device(Device {
                id: dev_id.clone(),
                name: nm.clone(),
                category: "Custom".into(),
                prefix: prefix.get_untracked(),
                symbol: sid,
                footprint: (!fp.is_empty()).then(|| fp.clone()),
                value: nm.clone(),
                ..Default::default()
            });
        });
        app.dialog.set(None);
        app.ok(format!("Created device {nm} with {} pins", list.len()));
        app.open(Tab::Device { id: dev_id });
    };
    view! {
        <Modal title="IC Symbol Wizard".into() wide=true>
            <div class="wizard">
                <div class="form">
                    <label class="field"><span>"Part name"</span><input type="text" prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev)) /></label>
                    <label class="field"><span>"Designator prefix"</span><input type="text" prop:value=move || prefix.get() on:input=move |ev| prefix.set(event_target_value(&ev)) /></label>
                    <label class="field"><span>"Footprint"</span>
                        <select on:change=move |ev| footprint.set(event_target_value(&ev))>
                            <option value="">"(none)"</option>
                            {fps.into_iter().map(|(id, n)| { let sel = id == footprint.get_untracked(); view! { <option value=id selected=sel>{n}</option> } }).collect_view()}
                        </select>
                    </label>
                    <label class="field col"><span>"Pins — one per line: number, name, type (in, out, io, power, power_out, oc, nc, passive). Paste straight from a datasheet table."</span>
                        <textarea rows="12" prop:value=move || pins.get() on:input=move |ev| pins.set(event_target_value(&ev))></textarea>
                    </label>
                </div>
                <div class="wizard-preview">
                    <div class="preview light" inner_html=move || preview.get()></div>
                    <p class="hint">{move || format!("{} pins; first half on the left, rest on the right.", parsed.with(|p| p.len()))}</p>
                </div>
            </div>
            <div class="modal-foot"><button on:click=move |_| app.dialog.set(None)>"Cancel"</button><button class="primary" on:click=create><Icon name="wand" />"Create Device"</button></div>
        </Modal>
    }
}
