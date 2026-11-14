//! The QuantRule desktop shell: a window over the gateway URL.
//!
//! The shell owns no IPC command, opens no local data file, and embeds no
//! engine. The window loads the bundled bootstrap page, which remembers the
//! gateway URL and performs the release update check; when the gateway is
//! reachable it navigates the same window to it. The built Svelte client is
//! bundled only as the disconnected fallback. See `README.md` for the boundary,
//! the smoke tests, and the build matrix.
//!
//! The one-way dependency rule still holds: this crate is outside the Rust
//! workspace and imports no `quantrule-*` crate. The gateway is the only
//! compute path.

#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = tauri::Builder::default().run(tauri::generate_context!()) {
        eprintln!("quantrule desktop shell failed to start: {error}");
        std::process::exit(1);
    }
}
