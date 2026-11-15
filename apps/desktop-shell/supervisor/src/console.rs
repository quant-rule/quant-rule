//! Keep GUI-launched children off the Windows console.
//!
//! A console-subsystem child started by a windows-subsystem parent otherwise
//! gets a console window of its own. The supervisor and the services are
//! console binaries, and a packaged desktop app must not flash a terminal for
//! each one. `CREATE_NO_WINDOW` is a documented `Command` flag, so this needs no
//! unsafe code and no crate.

use std::process::Command;

/// The Windows flag that suppresses a new console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Suppress the console window for a child on Windows; do nothing elsewhere.
pub fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = command;
    }
}
