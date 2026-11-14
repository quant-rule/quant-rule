//! An in-process HTTP fixture for the shell's printed-output tests.
//!
//! The fixture owns a `TcpListener` on an ephemeral loopback port and answers
//! the documented `/api/v1` surface. It records every request so a test can
//! prove the shell sent the method, path, profile header, and body the contract
//! requires. It reads no repository data and uses no wall clock in a payload.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{Value, json};

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

/// The mutable fixture state.
struct FixtureState {
    profiles: Vec<Value>,
    requests: Vec<Request>,
    marks: Vec<Value>,
    run_detail: Value,
    artifact_bytes: Vec<u8>,
    next_profile: u64,
}

/// The response a handler builds.
struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    fn json(status: u16, value: &Value) -> Self {
        Self {
            status,
            content_type: "application/json",
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

    fn bytes(status: u16, content_type: &'static str, body: Vec<u8>) -> Self {
        Self {
            status,
            content_type,
            body,
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
        )
    }

    /// Start the fixture with the browser's run-detail values.
    ///
    /// The values mirror `apps/web/e2e/run-detail.e2e.ts`; the shell must print
    /// the same numbers and codes for the same profile.
    pub fn start_parity() -> Self {
        Self::start_with(
            json!({
                "run_id": run_id(),
                "status": "succeeded",
                "owner": "default",
                "stages": [
                    { "name": "data", "state": "done", "elapsed_ms": 1_200 },
                    { "name": "model", "state": "done", "elapsed_ms": 2_400 },
                    { "name": "backtest", "state": "done", "elapsed_ms": 3_600 },
                    { "name": "evidence", "state": "done", "elapsed_ms": 120 },
                ],
                "provenance": {
                    "data_version": data_version(),
                    "config_hash": config_hash(),
                    "model_version": "M1.0",
                    "image_digest": image_digest(),
                },
                "artifacts": [
                    { "name": "results.json", "bytes": 2_048, "sha256": digest('f') }
                ],
                "digest": canonical_digest(),
                "error": null,
            }),
            Vec::new(),
        )
    }

    fn start_with(run_detail: Value, artifact_bytes: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the fixture binds");
        let address = listener.local_addr().expect("the fixture has an address");
        listener
            .set_nonblocking(true)
            .expect("the fixture listener is non-blocking");
        let state = Arc::new(Mutex::new(FixtureState {
            profiles: vec![
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
            ],
            requests: Vec::new(),
            marks: Vec::new(),
            run_detail,
            artifact_bytes,
            next_profile: 0,
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
                if let Some(request) = read_request(&mut stream) {
                    let response = {
                        let mut state = state.lock().expect("the fixture lock");
                        state.requests.push(request.clone());
                        handle(&mut state, &request)
                    };
                    let _ = write_response(&mut stream, response);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2));
            }
            Err(_) => break,
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4_096];
    loop {
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    let header_end = buffer.windows(4).position(|window| window == b"\r\n\r\n")? + 4;
    let head = std::str::from_utf8(&buffer[..header_end]).ok()?;
    let mut lines = head.lines();
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_owned();
    let target = parts.next()?.to_owned();
    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    let length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = buffer[header_end..].to_vec();
    while body.len() < length {
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(length);
    let (path, query) = split_target(&target);
    Some(Request {
        method,
        path,
        query,
        headers,
        body,
    })
}

fn write_response(stream: &mut TcpStream, response: Response) -> std::io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        409 => "Conflict",
        _ => "Internal Server Error",
    };
    let head = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(&response.body)?;
    stream.flush()
}

fn split_target(target: &str) -> (String, BTreeMap<String, String>) {
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    };
    let path = percent_decode(path);
    let path = path.strip_prefix("/api/v1").unwrap_or(&path).to_owned();
    let mut values = BTreeMap::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        values.insert(percent_decode(name), percent_decode(value));
    }
    (path, values)
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let high = hex(bytes[index + 1]);
                let low = hex(bytes[index + 2]);
                match (high, low) {
                    (Some(high), Some(low)) => {
                        decoded.push(high * 16 + low);
                        index += 3;
                    }
                    _ => {
                        decoded.push(bytes[index]);
                        index += 1;
                    }
                }
            }
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn handle(state: &mut FixtureState, request: &Request) -> Response {
    let path = request.path.as_str();
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if path == "/health" && request.method == "GET" {
        return Response::json(
            200,
            &json!({
                "v": "quantrule.api/v1",
                "state": "degraded",
                "services": [
                    { "name": "postgres", "state": "healthy", "detail": null },
                    { "name": "minio", "state": "degraded", "detail": "slow" },
                ],
                "queue_depth": 2,
                "workers": 1,
                "image_digest": image_digest(),
            }),
        );
    }
    if path == "/version" && request.method == "GET" {
        return Response::json(
            200,
            &json!({
                "product": "1.0.0",
                "api": "quantrule.api/v1",
                "image_digest": image_digest(),
                "contracts": [["api", "quantrule.api/v1"]],
            }),
        );
    }

    if !matches!(path, "/profiles" | "/health" | "/version") {
        let profile = request.header("x-quantrule-profile").unwrap_or("");
        if !state
            .profiles
            .iter()
            .any(|item| item["profile_id"].as_str() == Some(profile))
        {
            return Response::error(
                404,
                "profile.not_found",
                &format!("profile {profile} is unknown"),
            );
        }
    }

    match (request.method.as_str(), segments.as_slice()) {
        ("GET", ["profiles"]) => Response::json(200, &Value::Array(state.profiles.clone())),
        ("POST", ["profiles"]) => {
            let body = request.json();
            let id = body["profile_id"].as_str().unwrap_or("").to_owned();
            if let Some(existing) = state.profiles.iter().find(|item| item["profile_id"] == id) {
                return Response::json(200, existing);
            }
            state.next_profile += 1;
            let created = json!({
                "profile_id": id,
                "name": body["name"],
                "datasets": 0,
                "runs": 0,
                "simulations": 0,
            });
            state.profiles.push(created.clone());
            Response::json(201, &created)
        }
        ("GET", ["catalog", "datasets"]) => {
            Response::json(200, &json!([dataset_summary("candles.csv")]))
        }
        ("POST", ["catalog", "datasets"]) | ("POST", ["catalog", "datasets", "tolerant"]) => {
            let name = request
                .query
                .get("name")
                .cloned()
                .unwrap_or_else(|| String::from("upload.csv"));
            let mut record = json!({
                "summary": dataset_summary(&name),
                "object_key": format!("datasets/{}", quantrule_api::Digest::sha256(&request.body)),
                "sha256": data_version(),
                "quality": null,
                "created_at": "2026-11-14T00:00:00Z",
            });
            if path.ends_with("/tolerant") {
                record["quality"] = json!({
                    "rows_read": 8,
                    "accepted": 4,
                    "rejected": 4,
                    "duplicates": 1,
                    "out_of_order": 1,
                    "invalid": 2,
                    "skipped": [
                        { "line": 4, "reason": "duplicate" },
                        { "line": 5, "reason": "misaligned" },
                    ],
                });
            }
            Response::json(201, &record)
        }
        ("GET", ["catalog", "datasets", _]) => Response::json(
            200,
            &json!({
                "summary": dataset_summary("candles.csv"),
                "object_key": format!("datasets/{}", data_version()),
                "sha256": data_version(),
                "quality": null,
                "created_at": "2026-11-14T00:00:00Z",
            }),
        ),
        ("GET", ["catalog", "configs"]) => Response::json(
            200,
            &json!([
                {
                    "config_id": "cfg_abc",
                    "name": "run",
                    "origin": "repository",
                    "valid": true,
                    "message": null,
                    "config_hash": config_hash(),
                },
                {
                    "config_id": "cfg_bad",
                    "name": "broken",
                    "origin": "upload",
                    "valid": false,
                    "message": "portfolio.initial_cash must be positive",
                    "config_hash": null,
                }
            ]),
        ),
        ("GET", ["runs"]) => Response::json(
            200,
            &json!([{
                "run_id": run_id(),
                "status": "succeeded",
                "config_id": "cfg_abc",
                "data_version": data_version(),
                "submitted_at": "2026-11-13T12:00:00Z",
                "elapsed_ms": 1_200,
            }]),
        ),
        ("POST", ["runs"]) => Response::json(
            202,
            &json!({ "run_id": run_id(), "status": "queued", "created": true }),
        ),
        ("GET", ["runs", id]) if id.starts_with("sha256:9") => {
            Response::error(404, "run.not_found", "run is unknown")
        }
        ("GET", ["runs", _]) => Response::json(200, &state.run_detail),
        ("POST", ["runs", _, "cancel"]) => {
            let mut detail = state.run_detail.clone();
            detail["status"] = json!("cancelled");
            Response::json(202, &detail)
        }
        ("GET", ["runs", _, "digest"]) => Response::json(
            200,
            &json!({ "run_id": run_id(), "digest": canonical_digest() }),
        ),
        ("GET", ["runs", _, "artifacts", ..]) => Response::bytes(
            200,
            "application/octet-stream",
            state.artifact_bytes.clone(),
        ),
        ("GET", ["inspect", "candles"]) => Response::json(200, &dataset_summary("candles.csv")),
        ("GET", ["inspect", "timeframes"]) => Response::json(
            200,
            &json!({
                "data_version": data_version(),
                "timeframes": [
                    { "timeframe": "1m", "bars": 3, "partial_buckets": 0, "first_bar": 1_704_067_200, "last_bar": 1_704_067_320 },
                    { "timeframe": "5m", "bars": 1, "partial_buckets": 1, "first_bar": 1_704_067_200, "last_bar": 1_704_067_200 },
                    { "timeframe": "15m", "bars": 0, "partial_buckets": 1, "first_bar": null, "last_bar": null },
                ],
            }),
        ),
        ("GET", ["inspect", "events"]) => Response::json(
            200,
            &json!({
                "data_version": data_version(),
                "timeframe": request.query.get("timeframe").cloned().unwrap_or_default(),
                "events": [
                    { "type": 1, "source": 2, "bar": 10 },
                    { "type": 2, "source": 3, "bar": 11 },
                ],
            }),
        ),
        ("GET", ["simulation", "runs"]) => Response::json(
            200,
            &json!([
                {
                    "sim_id": "sim-2",
                    "premise": "second",
                    "status": "marking",
                    "windows": 4,
                    "marks": 2,
                },
                {
                    "sim_id": "sim-1",
                    "premise": "first",
                    "status": "draft",
                    "windows": 3,
                    "marks": 0,
                }
            ]),
        ),
        ("POST", ["simulation", "runs"]) => Response::json(201, &json!({ "sim_id": "sim-3" })),
        ("GET", ["simulation", "runs", _]) => Response::json(
            200,
            &json!({
                "sim_id": "sim-1",
                "premise": "first",
                "status": "draft",
                "inputs_digest": config_hash(),
                "windows": 3,
                "marks": 0,
            }),
        ),
        ("POST", ["simulation", "runs", _, "windows"]) => Response::json(
            200,
            &json!({
                "sim_id": "sim-1",
                "windows": [
                    { "window": 1, "start": 0, "end": 120 },
                    { "window": 2, "start": 121, "end": 240 },
                ],
            }),
        ),
        ("POST", ["simulation", "runs", _, "marks"]) => {
            let body = request.json();
            let mark_id = format!("mark-{}", state.marks.len() + 1);
            let mut mark = body;
            mark["mark_id"] = json!(mark_id);
            mark["marked_time_utc"] = json!("2026-11-14T10:00:00Z");
            mark["origin"] = json!("human");
            state.marks.push(mark);
            Response::json(201, &json!({ "mark_id": mark_id }))
        }
        ("GET", ["simulation", "runs", _, "marks"]) => {
            let origin = request.query.get("origin");
            let marks: Vec<Value> = state
                .marks
                .iter()
                .filter(|mark| {
                    origin.is_none_or(|origin| mark["origin"].as_str() == Some(origin.as_str()))
                })
                .cloned()
                .collect();
            Response::json(200, &json!({ "sim_id": "sim-1", "marks": marks }))
        }
        ("POST", ["simulation", "runs", _, "oracle"]) => Response::json(
            200,
            &json!({
                "sim_id": "sim-1",
                "marks": [
                    {
                        "mark_id": "oracle-1",
                        "chart": "w1",
                        "bar": 12,
                        "marked_time_utc": "2026-11-14T10:00:00Z",
                        "action": "entry",
                        "quality": "good",
                        "side": "long",
                        "origin": "oracle",
                        "note": ""
                    }
                ],
            }),
        ),
        ("POST", ["simulation", "runs", _, "check"]) => Response::json(
            200,
            &json!({ "sim_id": "sim-1", "events": 31, "verdict": "pass" }),
        ),
        ("POST", ["simulation", "runs", _, "decision"]) => {
            let body = request.json();
            Response::json(
                201,
                &json!({
                    "sim_id": "sim-1",
                    "decision": body["decision"],
                    "note": body["note"],
                }),
            )
        }
        _ => Response::error(
            404,
            "test.unhandled",
            &format!("{} {} is not fixtured", request.method, path),
        ),
    }
}

fn dataset_summary(name: &str) -> Value {
    json!({
        "data_version": data_version(),
        "name": name,
        "candles": 3,
        "first_bar": 1_704_067_200,
        "last_bar": 1_704_067_320,
        "bytes": 96,
    })
}

fn run_detail(artifact_digest: &str, artifact_bytes: u64) -> Value {
    json!({
        "run_id": run_id(),
        "status": "succeeded",
        "owner": "default",
        "stages": [
            { "name": "data", "state": "done", "elapsed_ms": 1_200 },
            { "name": "model", "state": "done", "elapsed_ms": 2_400 },
            { "name": "backtest", "state": "done", "elapsed_ms": 3_600 },
            { "name": "evidence", "state": "done", "elapsed_ms": 120 },
        ],
        "provenance": {
            "data_version": data_version(),
            "config_hash": config_hash(),
            "model_version": "M1.0",
            "image_digest": image_digest(),
        },
        "artifacts": [
            { "name": "results.json", "bytes": artifact_bytes, "sha256": artifact_digest }
        ],
        "digest": canonical_digest(),
        "error": null,
    })
}
