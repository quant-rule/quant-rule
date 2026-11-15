//! The supervisor's diagnostic record.
//!
//! `ports.json` is written for a human debugging a failed start. It is not a
//! control channel: the shell never reads it and no service depends on it. A
//! write is atomic on Unix and best-effort on Windows, where a rename cannot
//! replace an existing file; a torn diagnostic file is survivable, a torn
//! control surface would not be.

use std::fs;
use std::io;
use std::path::Path;

const STATUS_DIR: &str = "supervisor";
const STATUS_FILE: &str = "ports.json";
const TEMP_PREFIX: &str = ".tmp-";

/// One service in the diagnostic record.
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    /// The service name, such as `qr-gateway`.
    pub name: &'static str,
    /// The loopback port, or `None` for the worker.
    pub port: Option<u16>,
    /// The guard process, which is the parent of the service process.
    pub pid: u32,
    /// How many times this service was restarted.
    pub restarts: u32,
}

/// The record written after every state change.
#[derive(Debug, Clone)]
pub struct Status {
    /// `desktop` or `external`.
    pub profile: &'static str,
    /// `starting`, `ready`, `stopped`, or `failed`.
    pub state: &'static str,
    /// The control address.
    pub control: String,
    /// The gateway URL the shell should open.
    pub gateway: String,
    /// The six sidecars.
    pub services: Vec<ServiceStatus>,
}

/// Create the status directory and remove a previous run's temporary files.
///
/// Only files with the supervisor's own temporary prefix are removed, and only
/// inside its own directory. The content-addressed store and the SQLite file
/// belong to other components and are never touched here.
///
/// # Errors
///
/// Returns the I/O error when the directory cannot be created.
pub fn prepare(data_dir: &Path) -> io::Result<()> {
    let directory = data_dir.join(STATUS_DIR);
    fs::create_dir_all(&directory)?;
    if let Ok(entries) = fs::read_dir(&directory) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with(TEMP_PREFIX) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

/// Write the diagnostic record.
///
/// # Errors
///
/// Returns the I/O error when the directory or the file cannot be written.
pub fn write(data_dir: &Path, status: &Status) -> io::Result<()> {
    let directory = data_dir.join(STATUS_DIR);
    fs::create_dir_all(&directory)?;
    let final_path = directory.join(STATUS_FILE);
    let temp_path = directory.join(format!("{TEMP_PREFIX}{}", std::process::id()));
    fs::write(&temp_path, render(status))?;
    let _ = fs::remove_file(&final_path);
    fs::rename(&temp_path, &final_path)
}

fn render(status: &Status) -> String {
    let mut text = String::new();
    text.push_str("{\n");
    text.push_str(&format!("  \"profile\": \"{}\",\n", status.profile));
    text.push_str(&format!("  \"state\": \"{}\",\n", status.state));
    text.push_str(&format!("  \"control\": \"{}\",\n", status.control));
    text.push_str(&format!("  \"gateway\": \"{}\",\n", status.gateway));
    text.push_str("  \"services\": [\n");
    for (index, service) in status.services.iter().enumerate() {
        let port = match service.port {
            Some(port) => port.to_string(),
            None => "null".to_owned(),
        };
        text.push_str(&format!(
            "    {{\"name\": \"{}\", \"port\": {}, \"pid\": {}, \"restarts\": {}}}",
            service.name, port, service.pid, service.restarts
        ));
        if index + 1 < status.services.len() {
            text.push(',');
        }
        text.push('\n');
    }
    text.push_str("  ]\n}\n");
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_names_every_service_and_a_null_port() {
        let status = Status {
            profile: "desktop",
            state: "ready",
            control: "127.0.0.1:9".to_owned(),
            gateway: "http://127.0.0.1:8080".to_owned(),
            services: vec![
                ServiceStatus {
                    name: "qr-gateway",
                    port: Some(8080),
                    pid: 10,
                    restarts: 0,
                },
                ServiceStatus {
                    name: "qr-backtest",
                    port: None,
                    pid: 11,
                    restarts: 2,
                },
            ],
        };
        let text = render(&status);
        assert!(text.contains("\"name\": \"qr-gateway\", \"port\": 8080"));
        assert!(text.contains("\"name\": \"qr-backtest\", \"port\": null"));
        assert!(text.contains("\"restarts\": 2"));
        assert!(text.ends_with("]\n}\n"));
    }

    #[test]
    fn prepare_removes_only_its_own_temporary_files() {
        let root = std::env::temp_dir().join(format!("qr-status-{}", std::process::id()));
        let directory = root.join(STATUS_DIR);
        fs::create_dir_all(&directory).expect("dir");
        fs::write(directory.join(".tmp-1"), "stale").expect("stale");
        fs::write(directory.join("keep"), "keep").expect("keep");
        prepare(&root).expect("prepare");
        assert!(!directory.join(".tmp-1").exists());
        assert!(directory.join("keep").exists());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_second_write_replaces_the_first() {
        let root = std::env::temp_dir().join(format!("qr-status-write-{}", std::process::id()));
        let status = |state| Status {
            profile: "desktop",
            state,
            control: "127.0.0.1:9".to_owned(),
            gateway: "http://127.0.0.1:8080".to_owned(),
            services: Vec::new(),
        };
        write(&root, &status("starting")).expect("first");
        write(&root, &status("ready")).expect("second");
        let text = fs::read_to_string(root.join(STATUS_DIR).join(STATUS_FILE)).expect("read");
        assert!(text.contains("\"state\": \"ready\""));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
