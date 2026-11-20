//! The QuantRule desktop shell: a window over the gateway URL.
//!
//! The shell owns no IPC command, opens no local data file, and embeds no
//! engine. The window loads the bundled bootstrap page, which remembers the
//! gateway URL and performs the release update check; when the gateway is
//! reachable it navigates the same window to it. The built Svelte client is
//! bundled only as the disconnected fallback. See `README.md` for the boundary,
//! the smoke tests, and the build matrix.
//!
//! The one process the shell starts is the packaged `qr-desktop` supervisor,
//! from the resource directory, with standard input held open: closing it (on
//! exit, or when the shell dies) is the supervisor's stop signal. The shell
//! does not manage the six services, allocate a port, or read `ports.json`.
//! That is the supervisor's job, and it is why the shell still has no IPC
//! command and no Tauri plugin.
//!
//! The one-way dependency rule still holds: this crate is outside the Rust
//! workspace and imports no `quantrule-*` crate. The gateway is the only
//! compute path.

#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use tauri::{Manager, RunEvent};

/// The single process the shell owns.
struct Supervisor(Mutex<Option<Child>>);

fn main() {
    let builder = tauri::Builder::default().setup(|app| {
        if let Some(child) = spawn_supervisor(app.handle()) {
            app.manage(Supervisor(Mutex::new(Some(child))));
        }
        Ok(())
    });
    let app = match builder.build(tauri::generate_context!()) {
        Ok(app) => app,
        Err(error) => {
            eprintln!("quantrule desktop shell failed to start: {error}");
            std::process::exit(1);
        }
    };
    app.run(|app, event| {
        if matches!(event, RunEvent::Exit) {
            stop_supervisor(app);
        }
    });
}

/// Start the packaged supervisor once, if this package has one.
///
/// A development build has no supervisor beside the shell; that is not an
/// error, and the shell stays a window over the configured gateway. A
/// packaged build has `<resource>/sidecars/qr-desktop`, and the exe-relative
/// layouts cover a flat bundle and a locally staged directory.
fn spawn_supervisor(app: &tauri::AppHandle) -> Option<Child> {
    let mut bases = Vec::new();
    if let Ok(directory) = app.path().resource_dir() {
        bases.push(directory);
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        bases.push(parent.to_path_buf());
    }
    let name = format!("qr-desktop{}", std::env::consts::EXE_SUFFIX);
    for base in bases {
        for directory in [base.join("sidecars"), base.clone()] {
            let program = directory.join(&name);
            let mut command = Command::new(&program);
            command
                .arg("--profile")
                .arg("desktop")
                .arg("--resource-dir")
                .arg(&base)
                .arg("--parent-stdin")
                .stdin(Stdio::piped())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            hide_console(&mut command);
            match command.spawn() {
                Ok(child) => return Some(child),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    eprintln!("quantrule desktop shell: cannot start {program:?}: {error}");
                }
            }
        }
    }
    None
}

/// Suppress a console window for the supervisor on Windows. The packaged app
/// is a windows-subsystem binary; a console child would otherwise flash a
/// terminal once per launch.
#[cfg(windows)]
fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_command: &mut Command) {}

/// Stop the supervisor. Its control socket closes, and each guard ends its
/// service, so the shell never leaves a sidecar behind.
fn stop_supervisor(app: &tauri::AppHandle) {
    let Some(supervisor) = app.try_state::<Supervisor>() else {
        return;
    };
    let Ok(mut slot) = supervisor.0.lock() else {
        return;
    };
    if let Some(mut child) = slot.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}
