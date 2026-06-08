/// RanchSamples desktop — Tauri v2 application library.
///
/// The entry point is `run()`, called from `main.rs`. Keeping the logic in a
/// library crate lets us add integration tests later without duplicating the
/// setup code.
///
/// # Drag-to-DAW
///
/// `tauri-plugin-drag` exposes a `drag_files` command that the SvelteKit
/// front-end invokes via `@tauri-apps/api/core`:
///
/// ```js
/// import { invoke } from "@tauri-apps/api/core";
/// await invoke("plugin:drag|drag_files", {
///   items: [{ path: "/absolute/path/to/sample.wav", icon: null }],
///   image: null,
/// });
/// ```
///
/// The plugin handles the OS-level drag session; the SvelteKit layer only
/// needs the absolute file path (obtained from the catalog API).
///
/// # `dragDropEnabled: false`
///
/// `tauri.conf.json` sets `"dragDropEnabled": false` on the main window so
/// that Tauri's built-in HTML5 drag-drop interception is disabled. Without
/// this, a file dragged *from* the OS *into* the window would be swallowed
/// before the web code could handle it. The outbound plugin-drag flow
/// (*from* the app *to* the DAW) is unaffected by this setting.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_drag::init())
        .run(tauri::generate_context!())
        .expect("error while running RanchSamples desktop app");
}
