//! The container health probe.
//!
//! The runtime image deliberately carries no `curl` or CA package: the release
//! scan found Critical CVEs in the Debian `curl` build, and the services need
//! neither an external HTTP client nor a system trust store. The gateway
//! binary therefore answers one narrow probe mode that speaks HTTP/1.0 to a
//! loopback listener, checks the status and an optional body fragment, and
//! exits. It never starts the server and never reads the service configuration.

use std::fmt;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const IO_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_RESPONSE_BYTES: u64 = 64 * 1024;

/// A failure in the probe mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeError {
    /// The command line is missing an argument or has an extra one.
    Usage(String),
    /// The URL is not an absolute `http://` URL.
    InvalidUrl(String),
    /// No address could be reached.
    Connect(String),
    /// The request or response could not be read or written.
    Io(String),
    /// The response status was not 200.
    Status(String),
    /// The response body did not contain the expected fragment.
    Body(String),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message)
            | Self::InvalidUrl(message)
            | Self::Connect(message)
            | Self::Io(message)
            | Self::Status(message)
            | Self::Body(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ProbeError {}

/// Runs the probe mode and returns the process exit code.
///
/// The arguments are the values after `--healthcheck`: the URL, an optional
/// body fragment, and the optional `--print` flag. Exit `0` means the endpoint
/// answered `200` and the fragment matched, `1` means the probe failed, and `2`
/// means the arguments are wrong. `--print` writes the response body to stdout
/// so `qr-deploy check --runtime` can read `image_digest` without curl.
pub fn run<I>(args: I) -> i32
where
    I: IntoIterator<Item = String>,
{
    let mut positional = Vec::new();
    let mut print = false;
    for argument in args {
        match argument.as_str() {
            "--print" => print = true,
            value if value.starts_with("--") => {
                eprintln!("qr-gateway: unknown healthcheck flag {value:?}");
                return 2;
            }
            _ => positional.push(argument),
        }
    }
    let (url, expected) = match positional.as_slice() {
        [url] => (url.as_str(), None),
        [url, expected] => (url.as_str(), Some(expected.as_str())),
        _ => {
            eprintln!(
                "qr-gateway: usage: qr-gateway --healthcheck <http-url> [expected-body-fragment] [--print]"
            );
            return 2;
        }
    };
    match probe(url, expected) {
        Ok(body) => {
            if print {
                print!("{body}");
            }
            0
        }
        Err(error) => {
            eprintln!("qr-gateway: {error}");
            1
        }
    }
}

/// Probes one HTTP endpoint on the loopback interface.
///
/// The URL must be absolute and use `http://`; the health listeners are
/// plaintext on the private container network. A present `expected` fragment
/// must appear in the response body, so a process that returns `200` with a
/// degraded state still fails the probe. On success the response body is
/// returned; the probe itself never prints it.
pub fn probe(url: &str, expected: Option<&str>) -> Result<String, ProbeError> {
    let target = parse_url(url)?;
    let mut stream = connect(&target)?;
    let request = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nConnection: close\r\n\r\n",
        target.path, target.authority
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| ProbeError::Io(format!("cannot write the request: {error}")))?;
    let mut response = Vec::new();
    stream
        .take(MAX_RESPONSE_BYTES)
        .read_to_end(&mut response)
        .map_err(|error| ProbeError::Io(format!("cannot read the response: {error}")))?;
    let response = String::from_utf8_lossy(&response);
    let body = check_response(&response, expected)?;
    Ok(body.to_owned())
}

fn connect(target: &Target) -> Result<TcpStream, ProbeError> {
    let addresses = (target.host.as_str(), target.port)
        .to_socket_addrs()
        .map_err(|error| ProbeError::Connect(format!("cannot resolve {}: {error}", target.host)))?;
    let mut last = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(IO_TIMEOUT))
                    .map_err(|error| ProbeError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(IO_TIMEOUT))
                    .map_err(|error| ProbeError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last = Some(error),
        }
    }
    let detail = last.map_or_else(|| "no address".to_owned(), |error| error.to_string());
    Err(ProbeError::Connect(format!(
        "cannot connect to {}:{}: {detail}",
        target.host, target.port
    )))
}

fn check_response<'a>(response: &'a str, expected: Option<&str>) -> Result<&'a str, ProbeError> {
    let mut lines = response.split("\r\n");
    let status = lines.next().unwrap_or_default();
    let code = status
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| ProbeError::Status(format!("no HTTP status in {status:?}")))?;
    if code != 200 {
        return Err(ProbeError::Status(format!("unexpected HTTP status {code}")));
    }
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or_default();
    if let Some(expected) = expected
        && !body.contains(expected)
    {
        return Err(ProbeError::Body(format!(
            "the response body does not contain {expected:?}"
        )));
    }
    Ok(body)
}

struct Target {
    authority: String,
    host: String,
    port: u16,
    path: String,
}

fn parse_url(url: &str) -> Result<Target, ProbeError> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| ProbeError::InvalidUrl(format!("{url:?} is not an absolute http:// URL")))?;
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    if authority.is_empty() {
        return Err(ProbeError::InvalidUrl(format!("{url:?} has no host")));
    }
    if authority.contains('@') {
        return Err(ProbeError::InvalidUrl(format!(
            "{url:?} carries user information"
        )));
    }
    let (host, port) = split_host_port(authority)?;
    Ok(Target {
        authority: authority.to_owned(),
        host,
        port,
        path: path.to_owned(),
    })
}

fn split_host_port(authority: &str) -> Result<(String, u16), ProbeError> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, after) = rest.split_once(']').ok_or_else(|| {
            ProbeError::InvalidUrl(format!("unclosed IPv6 address in {authority:?}"))
        })?;
        let port = match after {
            "" => 80,
            value => {
                let value = value.strip_prefix(':').ok_or_else(|| {
                    ProbeError::InvalidUrl(format!("invalid authority {authority:?}"))
                })?;
                parse_port(value)?
            }
        };
        return Ok((host.to_owned(), port));
    }
    if let Some((host, value)) = authority.rsplit_once(':') {
        if host.contains(':') {
            return Err(ProbeError::InvalidUrl(format!(
                "IPv6 addresses in {authority:?} must be bracketed"
            )));
        }
        if host.is_empty() {
            return Err(ProbeError::InvalidUrl(format!("no host in {authority:?}")));
        }
        return Ok((host.to_owned(), parse_port(value)?));
    }
    Ok((authority.to_owned(), 80))
}

fn parse_port(value: &str) -> Result<u16, ProbeError> {
    value
        .parse::<u16>()
        .map_err(|_| ProbeError::InvalidUrl(format!("invalid port {value:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn parses_a_host_port_and_path() {
        let target = parse_url("http://127.0.0.1:8080/ready").expect("target");
        assert_eq!(target.host, "127.0.0.1");
        assert_eq!(target.port, 8080);
        assert_eq!(target.path, "/ready");
    }

    #[test]
    fn defaults_to_port_80_and_root_path() {
        let target = parse_url("http://gateway").expect("target");
        assert_eq!(target.host, "gateway");
        assert_eq!(target.port, 80);
        assert_eq!(target.path, "/");
    }

    #[test]
    fn accepts_bracketed_ipv6() {
        let target = parse_url("http://[::1]:9090/health").expect("target");
        assert_eq!(target.host, "::1");
        assert_eq!(target.port, 9090);
    }

    #[test]
    fn rejects_non_http_and_user_information() {
        assert!(matches!(
            parse_url("https://127.0.0.1/health"),
            Err(ProbeError::InvalidUrl(_))
        ));
        assert!(matches!(
            parse_url("http://user@127.0.0.1/health"),
            Err(ProbeError::InvalidUrl(_))
        ));
    }

    #[test]
    fn checks_the_status_and_optional_body_fragment() {
        let response =
            "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{\"state\":\"healthy\"}";
        check_response(response, None).expect("status");
        check_response(response, Some("\"state\":\"healthy\"")).expect("body");
        assert!(matches!(
            check_response("HTTP/1.1 503 Service Unavailable\r\n\r\n", None),
            Err(ProbeError::Status(_))
        ));
        assert!(matches!(
            check_response(response, Some("\"state\":\"degraded\"")),
            Err(ProbeError::Body(_))
        ));
    }

    #[test]
    fn a_loopback_probe_reads_the_response() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request = [0_u8; 512];
            let read = stream.read(&mut request).expect("read");
            assert!(
                String::from_utf8_lossy(&request[..read]).starts_with("GET /ready HTTP/1.0"),
                "request was {}",
                String::from_utf8_lossy(&request[..read])
            );
            stream
                .write_all(
                    b"HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{\"state\":\"healthy\"}",
                )
                .expect("write");
        });
        probe(
            &format!("http://{address}/ready"),
            Some("\"state\":\"healthy\""),
        )
        .expect("probe");
        server.join().expect("server");
    }
}
