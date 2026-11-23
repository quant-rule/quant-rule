//! An in-process HTTP fixture for the shell's printed-output tests.
//!
//! The fixture owns a `TcpListener` on an ephemeral loopback port and answers
//! the documented `/api/v1` surface. It records every request so a test can
//! prove the shell sent the method, path, profile header, and body the contract
//! requires. It reads no repository data and uses no wall clock in a payload.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{ErrorKind, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{Value, json};

mod artifact_fixture;
mod default_routes;
mod parity;
mod research;
mod stage_fixture;
mod wire;
#[allow(unused_imports)]
pub use artifact_fixture::{pack_body, pack_id, signal_body, signal_id};
use default_routes::run_detail;
#[allow(unused_imports)]
pub use stage_fixture::{
    client_parity_api, client_parity_view, portfolio_decisions_artifact_id, portfolio_stage_id,
    stage_detail, stage_events_body, stage_id, stage_report_body, strategy_decisions_body,
    strategy_stage_detail, strategy_stage_id,
};
use wire::{read_request, transient_accept, write_response};

/// The binary under test.
pub const BIN: &str = env!("CARGO_BIN_EXE_quantrule");

/// Build a wire digest from one repeated hexadecimal character.
pub fn digest(fill: char) -> String {
    format!("sha256:{}", fill.to_string().repeat(64))
}

/// The dataset fixture.
pub fn data_version() -> String {
    digest('c')
}

/// The run fixture.
pub fn run_id() -> String {
    digest('b')
}

/// The configuration fixture.
pub fn config_hash() -> String {
    digest('d')
}

/// The image fixture.
pub fn image_digest() -> String {
    digest('e')
}

/// The canonical digest fixture.
pub fn canonical_digest() -> String {
    digest('a')
}

/// One captured request.
#[derive(Debug, Clone)]
pub struct Request {
    /// HTTP method.
    pub method: String,
    /// Path after the `/api/v1` prefix, percent-decoded.
    pub path: String,
    /// Query values.
    pub query: BTreeMap<String, String>,
    /// Lower-case header names to values.
    pub headers: BTreeMap<String, String>,
    /// Raw body bytes.
    pub body: Vec<u8>,
}

impl Request {
    /// One header, by lower-case name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }

    /// The body parsed as JSON.
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("the request body is JSON")
    }
}

/// One scripted failure for a matching request.
struct Failure {
    method: String,
    path: String,
    /// `0` closes the connection without an answer; any other value answers it.
    status: u16,
    remaining: u32,
}

/// The mutable fixture state.
struct FixtureState {
    profiles: Vec<Value>,
    requests: Vec<Request>,
    marks: Vec<Value>,
    run_detail: Value,
    artifact_bytes: Vec<u8>,
    next_profile: u64,
    parity: Option<Value>,
    /// The bearer token the fixture requires, when a test sets one.
    token: Option<String>,
    /// Scripted transient failures, consumed in order.
    failures: Vec<Failure>,
}

/// The response a handler builds.
struct Response {
    status: u16,
    content_type: &'static str,
    headers: Vec<(&'static str, &'static str)>,
    body: Vec<u8>,
}

impl Response {
    fn json(status: u16, value: &Value) -> Self {
        Self {
            status,
            content_type: "application/json",
            headers: Vec::new(),
            body: serde_json::to_vec(value).expect("the fixture value is JSON"),
        }
    }

    fn error(status: u16, code: &str, message: &str) -> Self {
        Self::json(
            status,
            &json!({
                "v": "quantrule.api/v1",
                "error": { "code": code, "message": message },
            }),
        )
    }

    /// The gateway's refusal when the token is absent or wrong.
    fn unauthorized() -> Self {
        let mut response = Self::error(401, "auth.unauthorized", "a bearer token is required");
        response.headers.push(("www-authenticate", "Bearer"));
        response
    }

    fn bytes(status: u16, content_type: &'static str, body: Vec<u8>) -> Self {
        Self {
            status,
            content_type,
            headers: Vec::new(),
            body,
        }
    }

    /// Close the connection without writing a response.
    fn closed() -> Self {
        Self {
            status: 0,
            content_type: "application/json",
            headers: Vec::new(),
            body: Vec::new(),
        }
    }
}

/// A running fixture.
pub struct Fixture {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    state: Arc<Mutex<FixtureState>>,
    handle: Option<JoinHandle<()>>,
    home: PathBuf,
}

impl Fixture {
    /// Start the default fixture.
    pub fn start() -> Self {
        let artifact_bytes = b"{\"ok\":true}\n".to_vec();
        let artifact_digest = quantrule_api::Digest::sha256(&artifact_bytes).to_string();
        let artifact_length = artifact_bytes.len() as u64;
        Self::start_with(
            run_detail(&artifact_digest, artifact_length),
            artifact_bytes,
            None,
        )
    }

    /// Start the fixture with the shared client-parity responses.
    ///
    /// The run detail mirrors `apps/web/e2e/run-detail.e2e.ts`; every parity
    /// operation reads the same `tests/fixtures/client-parity/api.json` bytes
    /// the browser test reads.
    pub fn start_parity() -> Self {
        let fixture = client_parity_api();
        let run_detail = fixture["responses"]["run"].clone();
        let artifact_bytes = fixture["artifact"]["body"]
            .as_str()
            .expect("the fixture artifact body")
            .as_bytes()
            .to_vec();
        Self::start_with(run_detail, artifact_bytes, Some(fixture))
    }

    fn start_with(run_detail: Value, artifact_bytes: Vec<u8>, parity: Option<Value>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the fixture binds");
        let address = listener.local_addr().expect("the fixture has an address");
        listener
            .set_nonblocking(true)
            .expect("the fixture listener is non-blocking");
        let profiles = parity
            .as_ref()
            .and_then(|fixture| fixture["responses"]["profiles"].as_array())
            .cloned()
            .unwrap_or_else(|| {
                vec![
                    json!({
                        "profile_id": "default",
                        "name": "Default",
                        "datasets": 1,
                        "runs": 1,
                        "simulations": 1,
                    }),
                    json!({
                        "profile_id": "alpha",
                        "name": "Alpha",
                        "datasets": 1,
                        "runs": 0,
                        "simulations": 0,
                    }),
                ]
            });
        let state = Arc::new(Mutex::new(FixtureState {
            profiles,
            requests: Vec::new(),
            marks: Vec::new(),
            run_detail,
            artifact_bytes,
            next_profile: 0,
            parity,
            token: None,
            failures: Vec::new(),
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let handle = {
            let stop = Arc::clone(&stop);
            let state = Arc::clone(&state);
            thread::spawn(move || serve(listener, state, stop))
        };
        let home = std::env::temp_dir().join(format!(
            "quantrule-shell-{}-{:?}",
            std::process::id(),
            address.port()
        ));
        let _ = std::fs::remove_dir_all(&home);
        Self {
            address,
            stop,
            state,
            handle: Some(handle),
            home,
        }
    }

    /// The base URL, including `/api/v1`.
    pub fn url(&self) -> String {
        format!("http://{}/api/v1", self.address)
    }

    /// The shell's local state directory.
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// Run the shell against this fixture.
    pub fn run(&self, args: &[&str]) -> Output {
        Command::new(BIN)
            .args(args)
            .env("QUANTRULE_API_URL", self.url())
            .env("QUANTRULE_HOME", &self.home)
            .output()
            .expect("the quantrule binary runs")
    }

    /// Run the shell with one extra environment variable.
    pub fn run_with_env(&self, args: &[&str], name: &str, value: &str) -> Output {
        Command::new(BIN)
            .args(args)
            .env("QUANTRULE_API_URL", self.url())
            .env("QUANTRULE_HOME", &self.home)
            .env(name, value)
            .output()
            .expect("the quantrule binary runs")
    }

    /// Run the shell with no gateway URL, so the default base is used.
    pub fn run_without_api_url(&self, args: &[&str]) -> Output {
        Command::new(BIN)
            .args(args)
            .env_remove("QUANTRULE_API_URL")
            .env("QUANTRULE_HOME", &self.home)
            .output()
            .expect("the quantrule binary runs")
    }

    /// Run the interactive shell with a fixed input.
    pub fn run_interactive(&self, input: &str) -> Output {
        use std::process::Stdio;

        let mut child = Command::new(BIN)
            .env("QUANTRULE_API_URL", self.url())
            .env("QUANTRULE_HOME", &self.home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the quantrule binary starts");
        child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(input.as_bytes())
            .expect("the input is written");
        child
            .wait_with_output()
            .expect("the quantrule binary exits")
    }

    /// Every captured request, in order.
    pub fn requests(&self) -> Vec<Request> {
        self.state
            .lock()
            .expect("the fixture lock")
            .requests
            .clone()
    }

    /// A directory inside the fixture's temporary home for `run fetch`.
    pub fn fetch_directory(&self) -> PathBuf {
        self.home.join("fetch")
    }

    /// The last captured request.
    pub fn last_request(&self) -> Request {
        self.requests().pop().expect("the fixture saw a request")
    }

    /// Write the stored default profile.
    pub fn store_profile(&self, profile: &str) {
        std::fs::create_dir_all(&self.home).expect("the home directory is created");
        std::fs::write(self.home.join("profile"), format!("{profile}\n"))
            .expect("the profile is written");
    }

    /// Read the stored default profile.
    pub fn stored_profile(&self) -> Option<String> {
        std::fs::read_to_string(self.home.join("profile"))
            .ok()
            .map(|text| text.trim().to_owned())
    }

    /// Drop every captured request.
    pub fn clear_requests(&self) {
        self.state
            .lock()
            .expect("the fixture lock")
            .requests
            .clear();
    }

    /// Require `Authorization: Bearer <token>` on every gated route.
    ///
    /// `/health` and `/version` stay open, mirroring the gateway's bearer seam.
    pub fn require_token(&self, token: &str) {
        self.state.lock().expect("the fixture lock").token = Some(token.to_owned());
    }

    /// Answer the next `times` matching requests with `status` instead of the route.
    pub fn fail_next(&self, method: &str, path: &str, status: u16, times: u32) {
        assert!(status != 0, "use drop_next for a closed connection");
        self.state
            .lock()
            .expect("the fixture lock")
            .failures
            .push(Failure {
                method: method.to_owned(),
                path: path.to_owned(),
                status,
                remaining: times,
            });
    }

    /// Close the next `times` matching connections without an answer.
    pub fn drop_next(&self, method: &str, path: &str, times: u32) {
        self.state
            .lock()
            .expect("the fixture lock")
            .failures
            .push(Failure {
                method: method.to_owned(),
                path: path.to_owned(),
                status: 0,
                remaining: times,
            });
    }

    /// Add a profile to the list.
    pub fn add_profile(&self, id: &str, name: &str) {
        self.state
            .lock()
            .expect("the fixture lock")
            .profiles
            .push(json!({
                "profile_id": id,
                "name": name,
                "datasets": 0,
                "runs": 0,
                "simulations": 0,
            }));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

fn serve(listener: TcpListener, state: Arc<Mutex<FixtureState>>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                // Windows can let an accepted socket inherit the listener's
                // non-blocking flag; a first read would then see no data yet.
                let _ = stream.set_nonblocking(false);
                if let Some(request) = read_request(&mut stream) {
                    let response = {
                        let mut state = state.lock().expect("the fixture lock");
                        state.requests.push(request.clone());
                        if !authorized(&state, &request) {
                            Response::unauthorized()
                        } else if let Some(status) = take_failure(&mut state, &request) {
                            if status == 0 {
                                Response::closed()
                            } else {
                                Response::error(
                                    status,
                                    "service.unavailable",
                                    "the fixture refused the request",
                                )
                            }
                        } else {
                            handle(&mut state, &request)
                        }
                    };
                    if response.status == 0 {
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                    } else {
                        let _ = write_response(&mut stream, response);
                        let _ = stream.shutdown(std::net::Shutdown::Write);
                    }
                } else {
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                }
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2));
            }
            Err(error) if transient_accept(&error) => continue,
            Err(_) => break,
        }
    }
}

/// Consume one scripted failure and return its status, or `None`.
fn take_failure(state: &mut FixtureState, request: &Request) -> Option<u16> {
    let index = state.failures.iter().position(|failure| {
        failure.remaining > 0 && failure.method == request.method && failure.path == request.path
    })?;
    let failure = &mut state.failures[index];
    failure.remaining -= 1;
    let status = failure.status;
    if failure.remaining == 0 {
        state.failures.remove(index);
    }
    Some(status)
}

fn authorized(state: &FixtureState, request: &Request) -> bool {
    let Some(expected) = state.token.as_deref() else {
        return true;
    };
    if matches!(request.path.as_str(), "/health" | "/version") {
        return true;
    }
    let presented = format!("Bearer {expected}");
    request.header("authorization") == Some(presented.as_str())
}

fn handle(state: &mut FixtureState, request: &Request) -> Response {
    default_routes::handle(state, request)
}
