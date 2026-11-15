//! Lifecycle tests for the desktop supervisor.
//!
//! The six service binaries are replaced by `stub-sidecar`, copied to the six
//! names. The stub answers health and records the environment it received, so
//! the tests prove the launch contract without a database, an object store, or
//! a queue. The tests are the fast tier: no Docker, no network beyond loopback,
//! and no fixture outside the temporary directory.

use std::fs;
use std::io::Read;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use quantrule_desktop_supervisor::health;

const SERVICES: [&str; 6] = [
    "qr-gateway",
    "qr-data",
    "qr-model",
    "qr-backtest",
    "qr-evidence",
    "qr-simulation",
];

static SEQUENCE: AtomicU32 = AtomicU32::new(0);

struct Harness {
    root: PathBuf,
    sidecars: PathBuf,
    data: PathBuf,
    markers: PathBuf,
    gateway: u16,
    internal: u16,
    child: Option<Child>,
    last_stderr: String,
}

impl Harness {
    fn new(label: &str) -> Self {
        let sequence = SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "qr-desktop-{}-{}-{sequence}",
            std::process::id(),
            label
        ));
        let _ = fs::remove_dir_all(&root);
        let sidecars = root.join("sidecars");
        let data = root.join("data");
        let markers = root.join("markers");
        fs::create_dir_all(&sidecars).expect("sidecars");
        fs::create_dir_all(&data).expect("data");
        fs::create_dir_all(&markers).expect("markers");
        let (gateway, internal) = free_ports();
        for name in SERVICES {
            let target = sidecars.join(executable(name));
            fs::copy(stub(), &target).expect("copy stub");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut permissions = fs::metadata(&target).expect("metadata").permissions();
                permissions.set_mode(0o755);
                fs::set_permissions(&target, permissions).expect("mode");
            }
        }
        Self {
            root,
            sidecars,
            data,
            markers,
            gateway,
            internal,
            child: None,
            last_stderr: String::new(),
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_qr-desktop"));
        command
            .arg("--profile")
            .arg("desktop")
            .arg("--data-dir")
            .arg(&self.data)
            .arg("--sidecar-dir")
            .arg(&self.sidecars)
            .arg("--gateway-port")
            .arg(self.gateway.to_string())
            .arg("--internal-base")
            .arg(self.internal.to_string())
            .arg("--health-timeout-ms")
            .arg("15000")
            .arg("--shutdown-grace-ms")
            .arg("3000")
            .env("QR_STUB_MARKER_DIR", &self.markers)
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        command
    }

    fn spawn(&mut self) {
        self.spawn_with(&[]);
    }

    fn spawn_with(&mut self, environment: &[(&str, String)]) {
        let mut command = self.command();
        for (key, value) in environment {
            command.env(key, value);
        }
        self.child = Some(command.spawn().expect("spawn supervisor"));
    }

    fn spawn_with_parent_stdin(&mut self) {
        let mut command = self.command();
        command.arg("--parent-stdin").stdin(Stdio::piped());
        self.child = Some(command.spawn().expect("spawn supervisor"));
    }

    fn close_stdin(&mut self) {
        if let Some(child) = self.child.as_mut() {
            child.stdin.take();
        }
    }

    fn wait_ready(&self) {
        let address = format!("127.0.0.1:{}", self.gateway);
        wait_until(
            "the gateway became healthy",
            Duration::from_secs(15),
            || health::probe(&address, Duration::from_millis(300)),
        );
        wait_until(
            "all six stubs recorded a pid",
            Duration::from_secs(10),
            || SERVICES.iter().all(|name| self.pid(name).is_some()),
        );
        self.wait_record_ready();
    }

    fn wait_gateway_ready(&self, port: u16) {
        let address = format!("127.0.0.1:{port}");
        wait_until(
            "the fallback gateway became healthy",
            Duration::from_secs(15),
            || health::probe(&address, Duration::from_millis(300)),
        );
    }

    fn gateway_from_record(&self) -> u16 {
        let text = self.read_record();
        let marker = "\"gateway\": \"http://127.0.0.1:";
        let start = text.find(marker).expect("gateway in ports.json") + marker.len();
        let rest = &text[start..];
        let end = rest.find('"').expect("gateway port end");
        rest[..end].parse().expect("gateway port")
    }

    fn read_record(&self) -> String {
        fs::read_to_string(self.data.join("supervisor").join("ports.json")).expect("ports.json")
    }

    fn wait_record_ready(&self) {
        wait_until(
            "the supervisor recorded readiness",
            Duration::from_secs(10),
            || {
                self.data.join("supervisor").join("ports.json").is_file()
                    && self.read_record().contains("\"state\": \"ready\"")
            },
        );
    }

    fn pid(&self, name: &str) -> Option<u32> {
        fs::read_to_string(self.markers.join(format!("{name}.pid")))
            .ok()
            .and_then(|text| text.trim().parse().ok())
    }

    fn environment(&self, name: &str) -> String {
        fs::read_to_string(self.markers.join(format!("{name}.env"))).expect("env record")
    }

    fn wait_all_dead(&self) {
        wait_until("all six stubs exited", Duration::from_secs(10), || {
            SERVICES
                .iter()
                .filter_map(|name| self.pid(name))
                .all(|pid| !process_alive(pid))
        });
    }

    fn wait_exit(&mut self) -> Option<i32> {
        let child = self.child.as_mut()?;
        wait_until("the supervisor exited", Duration::from_secs(10), || {
            child.try_wait().ok().flatten().is_some()
        });
        let status = child.try_wait().ok().flatten();
        if let Some(stderr) = child.stderr.as_mut() {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            self.last_stderr = text;
        }
        self.child = None;
        status.and_then(|status| status.code())
    }

    fn kill_supervisor(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.child = None;
    }

    fn stderr(&self) -> &str {
        &self.last_stderr
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.kill_supervisor();
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn stub() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_stub-sidecar"))
}

fn executable(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

fn free_ports() -> (u16, u16) {
    // Serialize block selection so two concurrent tests can never be handed the
    // same range. The mutex is released before the supervisor binds, but the
    // next caller starts after this block, not inside it.
    static NEXT_BASE: Mutex<u16> = Mutex::new(0);
    let mut next = NEXT_BASE.lock().unwrap_or_else(|error| error.into_inner());
    if *next == 0 {
        *next = 20_000 + u16::try_from(std::process::id() % 64).unwrap_or(0) * 20;
    }
    loop {
        let base = *next;
        if base > 44_000 {
            panic!("no free port block");
        }
        let listeners: Vec<_> = (0..20)
            .filter_map(|offset| TcpListener::bind(("127.0.0.1", base + offset)).ok())
            .collect();
        let complete = listeners.len() == 20;
        drop(listeners);
        *next = base + 20;
        if complete {
            return (base, base + 10);
        }
    }
}

fn wait_until(label: &str, timeout: Duration, mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("timed out waiting until {label}");
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(windows)]
fn process_alive(pid: u32) -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output();
    match output {
        Ok(output) => String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()),
        Err(_) => false,
    }
}

#[test]
fn start_health_report_and_parent_stop() {
    let mut harness = Harness::new("start");
    harness.spawn_with_parent_stdin();
    harness.wait_ready();

    for name in SERVICES {
        let pid = harness
            .pid(name)
            .unwrap_or_else(|| panic!("{name} did not start"));
        assert!(process_alive(pid), "{name} is not running");
    }
    let gateway = harness.environment("qr-gateway");
    assert!(gateway.contains("QR_PROFILE=desktop"), "{gateway}");
    assert!(
        gateway.contains(&format!("QR_BIND=127.0.0.1:{}", harness.gateway)),
        "{gateway}"
    );
    assert!(
        gateway.contains(&format!(
            "QR_DATA_URL=http://127.0.0.1:{}",
            harness.internal
        )),
        "{gateway}"
    );
    let data_dir = gateway
        .lines()
        .find_map(|line| line.strip_prefix("QR_DATA_DIR="))
        .expect("QR_DATA_DIR in the child environment");
    assert!(!data_dir.is_empty(), "{gateway}");
    assert!(Path::new(data_dir).is_absolute(), "{gateway}");
    for name in SERVICES {
        let environment = harness.environment(name);
        assert!(
            environment.contains("QR_PROFILE=desktop"),
            "{name}: {environment}"
        );
        assert!(
            environment.contains(&format!("QR_DATA_DIR={data_dir}")),
            "{name}: {environment}"
        );
    }
    let record = harness.read_record();
    assert!(record.contains("\"state\": \"ready\""), "{record}");
    for name in SERVICES {
        assert!(record.contains(name), "{record}");
    }

    harness.close_stdin();
    assert_eq!(harness.wait_exit(), Some(0));
    harness.wait_all_dead();
}

#[test]
fn a_killed_supervisor_leaves_no_orphans_and_a_restart_binds_again() {
    let mut harness = Harness::new("orphan");
    harness.spawn();
    harness.wait_ready();
    let pids: Vec<_> = SERVICES
        .iter()
        .filter_map(|name| harness.pid(name))
        .collect();
    assert_eq!(pids.len(), SERVICES.len());

    harness.kill_supervisor();
    harness.wait_all_dead();
    for pid in pids {
        assert!(!process_alive(pid), "pid {pid} survived the supervisor");
    }

    harness.spawn();
    harness.wait_ready();
    harness.kill_supervisor();
    harness.wait_all_dead();
}

#[test]
fn a_crashed_service_is_restarted() {
    let mut harness = Harness::new("restart");
    let crash_once = harness.root.join("crashed-once");
    harness.spawn_with(&[
        ("QR_STUB_CRASH_SERVICE", "qr-data".to_owned()),
        (
            "QR_STUB_CRASH_ONCE",
            crash_once.to_string_lossy().into_owned(),
        ),
        ("QR_STUB_CRASH_AFTER_MS", "1200".to_owned()),
    ]);
    harness.wait_ready();
    let first = harness.pid("qr-data").expect("first qr-data pid");
    wait_until("qr-data restarted", Duration::from_secs(10), || {
        harness
            .pid("qr-data")
            .is_some_and(|current| current != first)
    });
    let second = harness.pid("qr-data").expect("second qr-data pid");
    assert!(process_alive(second));
    harness.wait_ready();
    wait_until("the restart was recorded", Duration::from_secs(10), || {
        harness.read_record().contains("\"restarts\": 1")
    });
    harness.kill_supervisor();
    harness.wait_all_dead();
}

#[test]
fn a_taken_gateway_port_falls_forward() {
    let mut harness = Harness::new("fallback");
    let holder = TcpListener::bind(("127.0.0.1", harness.gateway)).expect("hold gateway");
    harness.spawn();
    wait_until(
        "the supervisor used a fallback port",
        Duration::from_secs(15),
        || {
            harness.data.join("supervisor").join("ports.json").is_file()
                && harness.gateway_from_record() != harness.gateway
        },
    );
    let port = harness.gateway_from_record();
    harness.wait_gateway_ready(port);
    assert_eq!(port, harness.gateway + 1);
    drop(holder);
    harness.kill_supervisor();
    harness.wait_all_dead();
}

#[test]
fn a_missing_sidecar_fails_before_any_child_starts() {
    let mut harness = Harness::new("missing");
    fs::remove_file(harness.sidecars.join(executable("qr-simulation"))).expect("remove");
    harness.spawn();
    let code = harness.wait_exit();
    assert_eq!(code, Some(1));
    let stderr = harness.stderr();
    assert!(stderr.contains("qr-simulation"), "{stderr}");
    assert!(harness.pid("qr-gateway").is_none());
}

#[test]
fn a_blank_data_directory_is_diagnosed() {
    let harness = Harness::new("blank-data");
    let mut flag = Command::new(env!("CARGO_BIN_EXE_qr-desktop"));
    flag.arg("--profile")
        .arg("desktop")
        .arg("--data-dir")
        .arg("")
        .arg("--sidecar-dir")
        .arg(&harness.sidecars);
    let output = flag.output().expect("run");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("QR_DATA_DIR"), "{stderr}");
    assert!(stderr.contains("--data-dir"), "{stderr}");

    let mut variable = Command::new(env!("CARGO_BIN_EXE_qr-desktop"));
    variable
        .arg("--profile")
        .arg("desktop")
        .arg("--sidecar-dir")
        .arg(&harness.sidecars)
        .env("QR_DATA_DIR", "  ");
    let output = variable.output().expect("run");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("QR_DATA_DIR"), "{stderr}");
}

#[test]
fn the_external_profile_starts_nothing() {
    let harness = Harness::new("external");
    let mut command = Command::new(env!("CARGO_BIN_EXE_qr-desktop"));
    command
        .arg("--profile")
        .arg("external")
        .arg("--data-dir")
        .arg(&harness.data)
        .env("QR_STUB_MARKER_DIR", &harness.markers)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn");
    let status = child.wait().expect("wait");
    assert_eq!(status.code(), Some(0));
    assert!(harness.pid("qr-gateway").is_none());
}
