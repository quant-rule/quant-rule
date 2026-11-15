//! A test-only stand-in for one QuantRule service binary.
//!
//! The supervisor lifecycle tests copy this executable to the six service
//! names. It binds `QR_BIND` when one is set, answers `/health` with an
//! available report, and writes the environment the supervisor passed so the
//! test can prove the `QR_PROFILE=desktop` and `QR_DATA_DIR` contract reached
//! the child. `QR_STUB_CRASH_SERVICE` plus `QR_STUB_CRASH_ONCE` makes exactly
//! one service exit once, which exercises the restart path.
//!
//! The stub is never staged into an installer. It exists so the supervisor can
//! be tested before the embedded backends land (#693/#694).

#![forbid(unsafe_code)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

const HEALTH: &str = "{\"v\":\"quantrule.api/v1\",\"state\":\"healthy\",\"services\":[],\"queue_depth\":0,\"workers\":0,\"image_digest\":null}";

fn main() {
    let program = std::env::args().next().unwrap_or_default();
    let name = Path::new(&program)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "stub-sidecar".to_owned());
    write_record(&name);
    if let Some(bind) = std::env::var("QR_BIND")
        .ok()
        .filter(|value| !value.is_empty())
    {
        let listener = match TcpListener::bind(&bind) {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("stub {name}: cannot bind {bind}: {error}");
                std::process::exit(1);
            }
        };
        println!("stub {name}: listening on {bind}");
        thread::spawn(move || serve(listener));
    } else {
        println!("stub {name}: worker");
    }

    let crash_service = std::env::var("QR_STUB_CRASH_SERVICE").unwrap_or_default();
    if crash_service == name
        && let Some(path) = std::env::var_os("QR_STUB_CRASH_ONCE")
    {
        let path = PathBuf::from(path);
        if !path.exists() {
            let _ = std::fs::write(&path, format!("{name}\n"));
            let delay = std::env::var("QR_STUB_CRASH_AFTER_MS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(800);
            thread::sleep(Duration::from_millis(delay));
            eprintln!("stub {name}: exiting once to exercise the restart path");
            std::process::exit(3);
        }
    }

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

fn write_record(name: &str) {
    let Some(directory) = std::env::var_os("QR_STUB_MARKER_DIR") else {
        return;
    };
    let directory = PathBuf::from(directory);
    let _ = std::fs::create_dir_all(&directory);
    let _ = std::fs::write(
        directory.join(format!("{name}.pid")),
        format!("{}\n", std::process::id()),
    );
    let keys = [
        "QR_PROFILE",
        "QR_DATA_DIR",
        "QR_BIND",
        "QR_DESKTOP_CONTROL",
        "QR_STATIC_DIR",
        "QR_CONFIG_ROOT",
        "QR_DATA_URL",
        "QR_MODEL_URL",
        "QR_EVIDENCE_URL",
        "QR_SIMULATION_URL",
    ];
    let mut record = format!("name={name}\n");
    for key in keys {
        if let Ok(value) = std::env::var(key) {
            record.push_str(&format!("{key}={value}\n"));
        }
    }
    let _ = std::fs::write(directory.join(format!("{name}.env")), record);
}

fn serve(listener: TcpListener) {
    for stream in listener.incoming().flatten() {
        thread::spawn(move || answer(stream));
    }
}

fn answer(mut stream: TcpStream) {
    let mut request = [0_u8; 1024];
    let _ = stream.read(&mut request);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{HEALTH}",
        HEALTH.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}
