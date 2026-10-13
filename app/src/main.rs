//! EasyPRO web application.

mod icons;
mod js;
mod state;
mod storage;

use leptos::prelude::*;
use state::AppState;

#[component]
fn App() -> impl IntoView {
    let app = AppState::new();
    provide_context(app);
    view! { <div class="app">"EasyPRO"</div> }
}

fn main() {
    console_error_panic_hook::set_once();
    if let Some(boot) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("boot")) {
        boot.remove();
    }
    js::register_sw();
    leptos::mount::mount_to_body(App);
}
