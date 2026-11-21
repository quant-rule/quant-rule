//! Exercise the composed-tier preflight against a loopback gateway.
//!
//! The composed tier itself needs Docker and a real stack, so it is not part of
//! the fast tier. These tests exercise the checks the tier performs first and
//! seed the refusal paths: an unreachable gateway, an unhealthy one, and a
//! version without a product. The run-parity suite is replaced with a stub, so
//! no container is required.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

/// Serve one fixed health document and one fixed version document.
fn serve(health: &'static str, version: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            let mut buffer = [0_u8; 2048];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]);
            let body = if request.starts_with("GET /api/v1/health") {
                health
            } else {
                version
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
        }
    });
    format!("http://127.0.0.1:{port}/api/v1")
}

fn run(api_url: &str) -> Output {
    Command::new("bash")
        .arg(root().join("tools/composed.sh"))
        .current_dir(root())
        .env("QUANTRULE_API_URL", api_url)
        .env("QUANTRULE_TIMEOUT", "1")
        .env("QUANTRULE_VERIFY", "/bin/true")
        .env("QUANTRULE_QUEUE_CHECK", "0")
        .env("QUANTRULE_POSTGRES_CHECK", "0")
        .output()
        .expect("run composed.sh")
}

#[test]
fn a_healthy_gateway_passes_the_preflight() {
    let api = serve(
        r#"{"v":"quantrule.api/v1","state":"healthy"}"#,
        r#"{"api":"quantrule.api/v1","product":"1.1.0"}"#,
    );
    let output = run(&api);
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("run parity verified"));
}

#[test]
fn an_unhealthy_gateway_is_reported() {
    let api = serve(
        r#"{"v":"quantrule.api/v1","state":"degraded"}"#,
        r#"{"api":"quantrule.api/v1","product":"1.1.0"}"#,
    );
    let output = run(&api);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not 'healthy'"));
}

#[test]
fn a_missing_product_version_is_reported() {
    let api = serve(
        r#"{"v":"quantrule.api/v1","state":"healthy"}"#,
        r#"{"api":"quantrule.api/v1"}"#,
    );
    let output = run(&api);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no product version"));
}

#[test]
fn an_unreachable_gateway_is_reported() {
    let output = run("http://127.0.0.1:1/api/v1");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not reachable"));
}
