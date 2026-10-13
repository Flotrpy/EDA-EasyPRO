//! Line icons (24 x 24, stroke = currentColor). Drawn for this app so
//! there are no external assets or font downloads.

use leptos::prelude::*;

fn paths(name: &str) -> &'static str {
    match name {
        "folder" => r#"<path d="M3 6.5A1.5 1.5 0 0 1 4.5 5H9l2 2h8.5A1.5 1.5 0 0 1 21 8.5v9a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 17.5z"/>"#,
        "folder-open" => r#"<path d="M3 7V5.5A1.5 1.5 0 0 1 4.5 4H9l2 2h7.5A1.5 1.5 0 0 1 20 7.5V9"/><path d="M3 19l2.5-9h16L19 19z"/>"#,
        "new" => r#"<path d="M14 3H6.5A1.5 1.5 0 0 0 5 4.5v15A1.5 1.5 0 0 0 6.5 21h11a1.5 1.5 0 0 0 1.5-1.5V8z"/><path d="M14 3v5h5M12 11v6M9 14h6"/>"#,
        "save" => r#"<path d="M5 4h11l3 3v12a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1z"/><path d="M8 4v5h7V4M8 20v-6h8v6"/>"#,
        "undo" => r#"<path d="M9 7L4 12l5 5"/><path d="M4 12h10a5 5 0 0 1 0 10h-2"/>"#,
        "redo" => r#"<path d="M15 7l5 5-5 5"/><path d="M20 12H10a5 5 0 0 0 0 10h2"/>"#,
        "select" => r#"<path d="M5 3l14 8-6 1.5L10 19z"/>"#,
        "wire" => r#"<path d="M3 17h6V7h12"/><circle cx="9" cy="12" r="1.2" fill="currentColor"/>"#,
        "label" => r#"<path d="M3 16h18"/><path d="M5 12V7h7l3 2.5-3 2.5z"/>"#,
        "power" => r#"<path d="M12 20V9M7 9h10"/><path d="M9 5h6"/>"#,
        "ground" => r#"<path d="M12 4v8M6 12h12M8.5 15.5h7M11 19h2"/>"#,
        "noconnect" => r#"<path d="M7 7l10 10M17 7L7 17"/>"#,
        "text" => r#"<path d="M5 6V4h14v2M12 4v16M9 20h6"/>"#,
        "track" => r#"<path d="M3 18h6l6-8h6"/><circle cx="3" cy="18" r="1.6"/><circle cx="21" cy="10" r="1.6"/>"#,
        "via" => r#"<circle cx="12" cy="12" r="7"/><circle cx="12" cy="12" r="2.5"/>"#,
        "pour" => r#"<path d="M4 5h16v14H4z"/><path d="M4 12l8-7M8 19l12-11M14 19l6-5" opacity=".7"/>"#,
        "outline" => r#"<rect x="3.5" y="5.5" width="17" height="13" rx="1.5" stroke-dasharray="3 2"/>"#,
        "line" => r#"<path d="M4 19L20 5"/>"#,
        "measure" => r#"<path d="M3 15l12-12 6 6-12 12z"/><path d="M7 11l2 2M10 8l2 2M13 5l2 2"/>"#,
        "pin" => r#"<path d="M3 12h11"/><rect x="14" y="6" width="7" height="12" rx="1"/><circle cx="3" cy="12" r="1.3" fill="currentColor"/>"#,
        "rect" => r#"<rect x="4" y="6" width="16" height="12" rx="1"/>"#,
        "circle" => r#"<circle cx="12" cy="12" r="8"/>"#,
        "pad" => r#"<rect x="6" y="8" width="12" height="8" rx="2"/>"#,
        "tht" => r#"<circle cx="12" cy="12" r="7"/><circle cx="12" cy="12" r="3"/>"#,
        "fit" => r#"<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>"#,
        "zoom-in" => r#"<circle cx="11" cy="11" r="7"/><path d="M20 20l-4-4M8 11h6M11 8v6"/>"#,
        "zoom-out" => r#"<circle cx="11" cy="11" r="7"/><path d="M20 20l-4-4M8 11h6"/>"#,
        "layers" => r#"<path d="M12 4l9 5-9 5-9-5z"/><path d="M3 14l9 5 9-5"/>"#,
        "panel-left" => r#"<rect x="3" y="4" width="18" height="16" rx="1.5"/><path d="M9 4v16"/>"#,
        "panel-right" => r#"<rect x="3" y="4" width="18" height="16" rx="1.5"/><path d="M15 4v16"/>"#,
        "panel-bottom" => r#"<rect x="3" y="4" width="18" height="16" rx="1.5"/><path d="M3 14h18"/>"#,
        "settings" => r#"<circle cx="12" cy="12" r="3"/><path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1"/>"#,
        "help" => r#"<circle cx="12" cy="12" r="9"/><path d="M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.7.3-1 .9-1 1.7M12 17h.01"/>"#,
        "search" => r#"<circle cx="11" cy="11" r="7"/><path d="M20 20l-4-4"/>"#,
        "chip" => r#"<rect x="6" y="6" width="12" height="12" rx="1"/><path d="M9 3v3M12 3v3M15 3v3M9 18v3M12 18v3M15 18v3M3 9h3M3 12h3M3 15h3M18 9h3M18 12h3M18 15h3"/>"#,
        "footprint" => r#"<rect x="4" y="5" width="5" height="4" rx=".8"/><rect x="15" y="5" width="5" height="4" rx=".8"/><rect x="4" y="15" width="5" height="4" rx=".8"/><rect x="15" y="15" width="5" height="4" rx=".8"/>"#,
        "cube" => r#"<path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z"/><path d="M4 7.5l8 4.5 8-4.5M12 12v9"/>"#,
        "schematic" => r#"<path d="M3 12h4l2-4 3 8 2-4h7"/>"#,
        "pcb" => r#"<rect x="3" y="4" width="18" height="16" rx="1.5"/><path d="M7 8h4l3 4h3M7 16h6"/><circle cx="17" cy="12" r="1.2" fill="currentColor"/>"#,
        "check" => r#"<path d="M4 12.5l5 5L20 6.5"/>"#,
        "drc" => r#"<path d="M12 3l8 3v6c0 4.5-3.5 7.5-8 9-4.5-1.5-8-4.5-8-9V6z"/><path d="M8.5 12l2.5 2.5 4.5-5"/>"#,
        "export" => r#"<path d="M12 15V3M7 8l5-5 5 5"/><path d="M4 14v5a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-5"/>"#,
        "download" => r#"<path d="M12 3v12M7 10l5 5 5-5"/><path d="M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"/>"#,
        "import" => r#"<path d="M12 3v12M7 10l5 5 5-5"/><path d="M4 19h16"/>"#,
        "trash" => r#"<path d="M4 7h16M10 11v6M14 11v6"/><path d="M6 7l1 13h10l1-13M9 7V4h6v3"/>"#,
        "copy" => r#"<rect x="8" y="8" width="12" height="12" rx="1.5"/><path d="M16 8V5.5A1.5 1.5 0 0 0 14.5 4h-9A1.5 1.5 0 0 0 4 5.5v9A1.5 1.5 0 0 0 5.5 16H8"/>"#,
        "paste" => r#"<rect x="5" y="5" width="14" height="16" rx="1.5"/><path d="M9 5V3h6v2"/>"#,
        "rotate" => r#"<path d="M20 12a8 8 0 1 1-2.3-5.7"/><path d="M20 4v5h-5"/>"#,
        "mirror" => r#"<path d="M12 3v18" stroke-dasharray="2 2"/><path d="M9 7L4 17h5zM15 7l5 10h-5z"/>"#,
        "flip" => r#"<path d="M7 4v16M17 4v16"/><path d="M10 9l-3-3-3 3M14 15l3 3 3-3"/>"#,
        "grid" => r#"<path d="M4 4h16v16H4zM4 9.3h16M4 14.6h16M9.3 4v16M14.6 4v16"/>"#,
        "sun" => r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>"#,
        "moon" => r#"<path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z"/>"#,
        "book" => r#"<path d="M4 5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2z"/><path d="M4 19V5M19 19v2H6"/>"#,
        "x" => r#"<path d="M6 6l12 12M18 6L6 18"/>"#,
        "plus" => r#"<path d="M12 5v14M5 12h14"/>"#,
        "history" => r#"<path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5M12 7v5l3 2"/>"#,
        "keyboard" => r#"<rect x="2.5" y="6" width="19" height="12" rx="1.5"/><path d="M6 10h1M10 10h1M14 10h1M18 10h0M7 14h10"/>"#,
        "info" => r#"<circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7.5h.01"/>"#,
        "sync" => r#"<path d="M4 9a8 8 0 0 1 14-3l2 2M20 15a8 8 0 0 1-14 3l-2-2"/><path d="M20 4v4h-4M4 20v-4h4"/>"#,
        "list" => r#"<path d="M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01"/>"#,
        "number" => r#"<path d="M5 9h14M5 15h14M10 4L8 20M16 4l-2 16"/>"#,
        "sheet" => r#"<path d="M6 3h9l4 4v14H6z"/><path d="M15 3v4h4"/>"#,
        "board" => r#"<rect x="3" y="6" width="18" height="12" rx="1.5"/><path d="M7 10h3v4H7zM14 10h3"/>"#,
        "chevron-down" => r#"<path d="M6 9l6 6 6-6"/>"#,
        "chevron-right" => r#"<path d="M9 6l6 6-6 6"/>"#,
        "eye" => r#"<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>"#,
        "eye-off" => r#"<path d="M3 3l18 18M10.6 5.1A9.6 9.6 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-3.2 4.2M6.6 6.6C3.8 8.4 2 12 2 12s3.5 7 10 7c1.8 0 3.3-.5 4.7-1.3"/>"#,
        "lock" => r#"<rect x="5" y="11" width="14" height="10" rx="1.5"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>"#,
        "wand" => r#"<path d="M4 20L16 8M14 6l4 4"/><path d="M18 3v3M16.5 4.5h3M6 4v2M5 5h2"/>"#,
        "cpl" => r#"<path d="M12 21s-6-5.5-6-11a6 6 0 0 1 12 0c0 5.5-6 11-6 11z"/><circle cx="12" cy="10" r="2"/>"#,
        "palette" => r#"<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M7 10h10M7 14h6"/>"#,
        _ => r#"<circle cx="12" cy="12" r="2"/>"#,
    }
}

pub fn svg(name: &str) -> String {
    format!(
        r#"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{}</svg>"#,
        paths(name)
    )
}

#[component]
pub fn Icon(#[prop(into)] name: String) -> impl IntoView {
    view! { <span class="ic" inner_html=svg(&name)></span> }
}
