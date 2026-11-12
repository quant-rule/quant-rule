//! The QuantRule desktop binary.
//!
//! The binary is a thin Tauri host. Every command lives in the library and
//! calls the shared application service; the host owns only the window and the
//! user-facing startup error.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = quantrule_desktop_lib::run() {
        eprintln!("QuantRule could not start: {error}");
        eprintln!(
            "Start the app inside a graphical session and check that GTK 3 and WebKitGTK 4.1 are installed."
        );
        std::process::exit(1);
    }
}
