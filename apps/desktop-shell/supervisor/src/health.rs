//! A minimal readiness probe.
//!
//! The supervisor has no HTTP client dependency. It needs one fact: does this
//! loopback service answer `/health` with an available aggregate state? A raw
//! request over `std::net` is enough, and it keeps the package free of a crate
//! that would only be used for six requests on a local socket.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// The longest response body the probe reads. The health report is a few
/// hundred bytes; a larger body means this is not the service that was asked.
const MAX_BODY: usize = 8 * 1024;

/// Probe one `127.0.0.1:<port>` address.
///
/// A probe answers `true` only for an HTTP response whose aggregate state is
/// `healthy` or `degraded`. A process that is listening but reports `down` is
/// not ready: that is exactly the Postgres-backed stack before #693/#694, and
/// treating it as ready would fake a desktop profile.
#[must_use]
pub fn probe(address: &str, timeout: Duration) -> bool {
    let Ok(target) = address.parse::<std::net::SocketAddr>() else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&target, timeout) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let request = format!(
        "GET /health HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nAccept: application/json\r\n\r\n"
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                response.extend_from_slice(&buffer[..read]);
                if response.len() >= MAX_BODY {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let Ok(text) = std::str::from_utf8(&response) else {
        return false;
    };
    let Some((head, body)) = text.split_once("\r\n\r\n") else {
        return false;
    };
    if !head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200") {
        return false;
    }
    state_is_available(body)
}

fn state_is_available(body: &str) -> bool {
    let Some(start) = body.find("\"state\":\"") else {
        return false;
    };
    let rest = &body[start + "\"state\":\"".len()..];
    let Some(end) = rest.find('"') else {
        return false;
    };
    matches!(&rest[..end], "healthy" | "degraded")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    fn serve_once(response: &'static str) -> String {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        let address = listener.local_addr().expect("address").to_string();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 1024];
                let _ = stream.read(&mut buffer);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        address
    }

    #[test]
    fn a_healthy_report_is_ready() {
        let address = serve_once(
            "HTTP/1.1 200 OK\r\nContent-Length: 41\r\n\r\n{\"v\":\"v1\",\"state\":\"healthy\",\"services\":[]}",
        );
        assert!(probe(&address, Duration::from_millis(500)));
    }

    #[test]
    fn a_degraded_report_is_ready() {
        let address = serve_once(
            "HTTP/1.1 200 OK\r\nContent-Length: 40\r\n\r\n{\"v\":\"v1\",\"state\":\"degraded\",\"services\":[]}",
        );
        assert!(probe(&address, Duration::from_millis(500)));
    }

    #[test]
    fn a_down_report_is_not_ready() {
        let address = serve_once(
            "HTTP/1.1 200 OK\r\nContent-Length: 36\r\n\r\n{\"v\":\"v1\",\"state\":\"down\",\"services\":[]}",
        );
        assert!(!probe(&address, Duration::from_millis(500)));
    }

    #[test]
    fn an_error_status_is_not_ready() {
        let address = serve_once("HTTP/1.1 503 Service Unavailable\r\n\r\n");
        assert!(!probe(&address, Duration::from_millis(500)));
    }

    #[test]
    fn a_closed_port_is_not_ready() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        let address = listener.local_addr().expect("address").to_string();
        drop(listener);
        assert!(!probe(&address, Duration::from_millis(100)));
    }

    #[test]
    fn only_the_first_state_is_consulted() {
        assert!(!state_is_available(
            "{\"state\":\"down\",\"services\":[{\"state\":\"healthy\"}]}"
        ));
        assert!(state_is_available(
            "{\"state\":\"healthy\",\"services\":[{\"state\":\"down\"}]}"
        ));
        assert!(!state_is_available("{}"));
    }
}
