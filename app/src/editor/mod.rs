//! Editors shown in the document area.

pub mod canvas;
pub mod device;
pub mod footprint;
pub mod pcb;
pub mod sch;
pub mod symbol;
pub mod view3d;

use crate::state::{use_app, Tab};
use crate::ui::start::StartPage;
use canvas::{Canvas2d, Kind};
use leptos::prelude::*;

/// The active document.
#[component]
pub fn Workspace() -> impl IntoView {
    let app = use_app();
    let tab = Memo::new(move |_| app.tab());
    move || {
        let t = tab.get();
        match &t {
            Tab::Start => view! { <StartPage /> }.into_any(),
            Tab::View3d { board } => view! { <view3d::View3dPane board=*board /> }.into_any(),
            Tab::Device { id } => view! { <device::DevicePage id=id.clone() /> }.into_any(),
            _ => match Kind::from_tab(&t) {
                Some(k) => view! { <Canvas2d kind=k /> }.into_any(),
                None => view! { <div></div> }.into_any(),
            },
        }
    }
}
