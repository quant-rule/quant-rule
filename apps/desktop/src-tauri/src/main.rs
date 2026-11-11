//! The QuantRule desktop binary.
//!
//! The binary is a thin Tauri host. Every command lives in the library and
//! calls the shared application service; the host owns only the window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    quantrule_desktop_lib::run();
}
