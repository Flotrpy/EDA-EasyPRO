//! Start page: quick actions, recent projects and examples. No banners.

use crate::commands;
use crate::icons::Icon;
use crate::state::use_app;
use crate::ui::left::RecentProjects;
use leptos::prelude::*;

#[component]
fn Tile(id: &'static str, label: &'static str, icon: &'static str) -> impl IntoView {
    let app = use_app();
    view! {
        <button class="tile" on:click=move |_| commands::run(&app, id)>
            <span class="tile-icon"><Icon name=icon /></span>
            <span>{label}</span>
        </button>
    }
}

#[component]
pub fn StartPage() -> impl IntoView {
    let app = use_app();
    view! {
        <div class="page start">
            <div class="start-grid">
                <section class="card">
                    <h3>"Quick Start"</h3>
                    <div class="tiles">
                        <Tile id="file.new" label="New Project" icon="new" />
                        <Tile id="file.open" label="Open Project" icon="folder-open" />
                        <Tile id="file.example" label="Open Example" icon="book" />
                        <Tile id="tools.new_device" label="New Device" icon="chip" />
                        <Tile id="tools.ic_wizard" label="IC Symbol Wizard" icon="wand" />
                        <Tile id="file.import_fp" label="Import KiCad Footprint" icon="footprint" />
                        <Tile id="file.import_sym" label="Import KiCad Symbols" icon="schematic" />
                        <Tile id="settings.prefs" label="Settings" icon="settings" />
                        <Tile id="help.shortcuts" label="Shortcuts" icon="keyboard" />
                        <Tile id="help.about" label="About" icon="info" />
                    </div>
                </section>
                <section class="card">
                    <h3>"Recent Projects"</h3>
                    <RecentProjects />
                </section>
                <section class="card">
                    <h3>"Examples"</h3>
                    <div class="example-list">
                        {easypro_core::examples::all().into_iter().enumerate().map(|(i, (name, _))| view! {
                            <button class="example-item" on:click=move |_| crate::actions::open_example_at(&app, i)>
                                <Icon name="book" />
                                <span>{name}</span>
                            </button>
                        }).collect_view()}
                    </div>
                </section>
                <section class="card">
                    <h3>"Workflow"</h3>
                    <ol class="steps">
                        <li><b>"Schematic"</b>" — place parts from the Common Library (Shift+F), draw wires (W), label nets (N)."</li>
                        <li><b>"Update PCB"</b>" — Ctrl+U brings parts and nets into the board."</li>
                        <li><b>"Layout"</b>" — drag parts in, route with W (V for a via), add copper areas with E."</li>
                        <li><b>"Check"</b>" — run DRC, look at the board in 3D (Alt+3)."</li>
                        <li><b>"Fabricate"</b>" — export Gerber, drill, BOM and CPL for any fab (Ctrl+Alt+G)."</li>
                    </ol>
                </section>
                <section class="card">
                    <h3>"Your data"</h3>
                    <p>"Projects are saved in this browser automatically and to files you choose with Ctrl+S. Nothing is uploaded; there is no account."</p>
                    <p>"Files are plain JSON with stable ordering, so they work well with git. Every manual save also keeps a version you can restore from File → Version History."</p>
                    <p class="hint">{move || if app.project.with(|p| p.boards.iter().all(|b| b.schematic.components().count() == 0)) { "Current project is empty." } else { "" }}</p>
                </section>
            </div>
        </div>
    }
}
