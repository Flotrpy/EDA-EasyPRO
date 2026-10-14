# What users want fixed in EasyEDA Pro

Before writing code we looked at what people say about EasyEDA (Standard and
Pro) on the EasyEDA forum, EEVblog, r/easyeda (via a mirror and a topic
summary) and hobbyist forums. The recurring complaints, and what EasyPRO
does about each:

| Complaint | Source | EasyPRO's answer |
|---|---|---|
| Saving fails or loses work; projects only live in the vendor cloud, and even the desktop client syncs offline projects to it | GummySearch summary of r/easyeda ("Saving constantly fails"), EEVblog EasyEDA thread | Local-first: every edit is saved to IndexedDB in the browser within a second, Ctrl+S writes a real file (File System Access API or download). No account, nothing uploaded. Works offline (service worker). |
| No revision control in Pro | EasyEDA forum, "Project versioning" | Stable, pretty-printed JSON with sorted maps (diffs cleanly in git) plus a built-in version history: every save keeps a restorable version. |
| Slow on larger designs (≈700 parts: seconds per click) | EasyEDA forum, "Can not perform with more components" | Integer geometry in Rust/WebAssembly, R-tree spatial index for connectivity and DRC, WebGL2 rendering where pan/zoom only change shader uniforms. |
| Vendor lock-in to LCSC/JLCPCB | SimpleFOC forum, V1E forum, buyer guides | Fab presets for JLCPCB, PCBWay, OSH Park, Eurocircuits and generic Gerber X2. No ordering buttons or coupons. KiCad footprint/symbol import and KiCad netlist export. |
| Pro changed workflows and added bugs; Standard users found it a hassle | EEVblog EasyEDA thread | The layout copies EasyEDA Pro (title bar menus, toolbar, left project/library panel with vertical tabs, document tabs, right properties panel, bottom log) so the move is painless, but common actions are one key away and every command is in the Ctrl+K palette. |
| Hotkey settings don't save | EasyEDA forum, "Multiple Issues With Pro" | Every command can be rebound in Settings → Keyboard Shortcuts; bindings are stored locally and conflicts are flagged. |
| Rendering bugs (rotated parts split in half, opposite-side silkscreen showing through) | EasyEDA forum, "Multiple Issues With Pro" | One transform model shared by schematic, PCB and 3D with unit tests for rotation and mirroring; bottom-side layers are mapped explicitly. |
| Making your own parts is tedious | Several forum threads | IC Symbol Wizard turns a pasted pin list into a symbol and device; footprint editor copies the last pad size; device page shows symbol, footprint, 3D and pin mapping in one place. |
| Weak autorouter / routing help | Buyer guides | Interactive router with 45° shaping, live clearance checking and automatic choice of the clear bend; vias while routing (V). A full push-and-shove router and autorouter are not implemented yet. |
| Ads and promotions in the editor | EasyEDA start page (coupon banners, chat bubble, "Order PCB") | None. The start page has quick actions, recent projects and a short workflow guide. |

## Sources

- EasyEDA forum: [Multiple Issues With Pro](https://easyeda.com/forum/topic/Multiple-Issues-With-Pro-76527afe7fe34145b62526ca68aecc20),
  [Can not perform with more components](https://easyeda.com/forum/topic/Can-not-perform-with-more-components-bede1285efb844ea928aa2f034b9180c),
  [Project versioning](https://easyeda.com/forum/topic/Project-versioning-ed433516734240bfa28255d7337d3348),
  [Everything wrong with EasyEDA aka LCEDA](https://easyeda.com/forum/topic/Everything-wrong-with-EasyEDA-aka-LCEDA-124b0d348cfa4e39828a3db7f7065e2d),
  [All the problems with EasyEDA and why I'll switch](https://easyeda.com/forum/topic/All-the-problems-with-EasyEDA-and-why-I-ll-switch-to-Flux-AI-etc-ed439d0f90104d2980cd09aa14f83cd9)
- [EEVblog forum: EasyEDA?](https://www.eevblog.com/forum/eda/easyeda/)
- [GummySearch: r/easyeda](https://gummysearch.com/r/easyeda)
- [SimpleFOC community: Thoughts on EasyEDA](https://community.simplefoc.com/t/thoughts-on-easyeda/1302)
- [V1 Engineering forum: EasyEDA Pro or KiCad with JLC library](https://forum.v1e.com/t/easyeda-pro-or-kicad-with-jlclibrary/50256)

Some posts are from 2020–2023 and may not reflect the newest EasyEDA releases.

## Tech stack decision

Rust was requested and fits well:

- **Core in plain Rust** (`crates/core`): document model, connectivity, ERC,
  DRC, copper pours, file format. It runs and is tested natively, then
  compiles unchanged to WebAssembly.
- **UI with Leptos** (CSR): fine-grained reactivity keeps panels cheap to
  update while the canvases are drawn imperatively.
- **WebGL2 through `web-sys`** for both 2D editors and the 3D view, with
  our own small tessellator and shaders instead of a heavy engine.
- **Trunk** builds the static site, which GitHub Pages hosts for free.

Alternatives considered: a TypeScript app with a canvas library (faster to
start, but the request was Rust and geometry code benefits from Rust's
integer math and tests), and Bevy/egui (game-oriented immediate-mode UI that
would not look like EasyEDA without a lot of custom work).
