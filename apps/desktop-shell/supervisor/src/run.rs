//! The supervisor run loop.
//!
//! The order is deliberate: resolve the resource directory, check that all six
//! binaries are present before starting any of them, bind the control socket,
//! allocate ports, start the guards, wait for health, then watch. A partially
//! started package is a failed package, not a degraded one, so a missing
//! binary stops the start before a single process exists.

use std::fs;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::cli::{Config, Profile};
use crate::package;
use crate::ports::PortPlan;
use crate::status::{ServiceStatus, Status};
use crate::{health, ports, status};

/// How many times one service may be restarted before the profile is failed.
const MAX_RESTARTS: u32 = 4;
/// How often the main loop inspects its children.
const POLL_INTERVAL_MS: u64 = 250;
/// How long one readiness probe may take.
const PROBE_TIMEOUT_MS: u64 = 300;
/// The widest restart backoff.
const MAX_BACKOFF_MS: u64 = 2_000;

const SERVICES: [ServiceSpec; 6] = [
    ServiceSpec {
        name: "qr-gateway",
        program: "qr-gateway",
        kind: Kind::Gateway,
    },
    ServiceSpec {
        name: "qr-data",
        program: "qr-data",
        kind: Kind::Http,
    },
    ServiceSpec {
        name: "qr-model",
        program: "qr-model",
        kind: Kind::Http,
    },
    ServiceSpec {
        name: "qr-backtest",
        program: "qr-backtest",
        kind: Kind::Worker,
    },
    ServiceSpec {
        name: "qr-evidence",
        program: "qr-evidence",
        kind: Kind::Http,
    },
    ServiceSpec {
        name: "qr-simulation",
        program: "qr-simulation",
        kind: Kind::Http,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Gateway,
    Http,
    Worker,
}

#[derive(Debug, Clone, Copy)]
struct ServiceSpec {
    name: &'static str,
    program: &'static str,
    kind: Kind,
}

impl ServiceSpec {
    fn has_http(self) -> bool {
        !matches!(self.kind, Kind::Worker)
    }
}

/// Run the supervisor and return its process exit code.
#[must_use]
pub fn run(mut config: Config) -> i32 {
    if config.profile == Profile::External {
        println!(
            "qr-desktop {}: external profile, no sidecars started",
            env!("CARGO_PKG_VERSION")
        );
        return 0;
    }
    match prepare_data_dir(&config.data_dir) {
        Ok(directory) => config.data_dir = directory,
        Err(error) => {
            eprintln!("qr-desktop: {error}");
            return 1;
        }
    }
    match supervise(&config) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("qr-desktop: {error}");
            1
        }
    }
}

/// Resolve the desktop data directory to an absolute path.
///
/// #693's desktop profile requires a non-empty `QR_DATA_DIR` and rejects a
/// blank one. The supervisor refuses it earlier with a message that names both
/// the flag and the variable, and it makes the path absolute so a packaged app
/// launched from an arbitrary working directory passes a stable location.
fn prepare_data_dir(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err(crate::cli::DATA_DIR_REQUIRED.to_owned());
    }
    fs::create_dir_all(path).map_err(|error| {
        format!(
            "cannot create the data directory {}: {error}",
            path.display()
        )
    })?;
    fs::canonicalize(path).map_err(|error| {
        format!(
            "cannot resolve the data directory {}: {error}",
            path.display()
        )
    })
}

fn supervise(config: &Config) -> Result<(), String> {
    status::prepare(&config.data_dir)
        .map_err(|error| format!("cannot prepare {}: {error}", config.data_dir.display()))?;
    let sidecar_dir = resolve_sidecar_dir(config)?;
    let programs = sidecar_programs(&sidecar_dir)?;
    // A packaged run records the package that executed it. An explicit value
    // wins so the desktop/server parity harness can hold run identity constant;
    // otherwise the digest covers the six installed sidecars.
    let image_digest = match std::env::var("QR_IMAGE_DIGEST")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        Some(value) => value,
        None => package::digest(&programs)?,
    };
    let static_dir = config
        .resource_dir
        .as_ref()
        .map(|directory| directory.join("client"))
        .filter(|directory| directory.join("index.html").is_file());
    let config_root = std::env::var_os("QR_CONFIG_ROOT")
        .map(PathBuf::from)
        .or_else(|| {
            config
                .resource_dir
                .as_ref()
                .map(|directory| directory.join("config"))
                .filter(|directory| directory.is_dir())
        });
    let supervisor = std::env::current_exe()
        .map_err(|error| format!("cannot resolve the supervisor binary: {error}"))?;
    let control = Control::bind()?;
    let plan = allocate_with_retry(config)?;
    let gateway = format!("http://127.0.0.1:{}", plan.gateway);
    let spawner = Spawner {
        supervisor,
        data_dir: config.data_dir.clone(),
        control: control.address.clone(),
        plan,
        static_dir,
        config_root,
        image_digest,
    };
    let mut running = Vec::with_capacity(SERVICES.len());
    for (spec, program) in SERVICES.iter().zip(programs) {
        match spawner.spawn(*spec, &program) {
            Ok(child) => running.push(Running {
                spec: *spec,
                program,
                port: plan.port_for(spec.name),
                child,
                restarts: 0,
            }),
            Err(error) => {
                stop(
                    &mut running,
                    &control,
                    Duration::from_millis(config.shutdown_grace_ms),
                );
                return Err(error);
            }
        }
    }
    println!(
        "qr-desktop {}: desktop profile, data {}, sidecars {}, control {}, gateway {}",
        env!("CARGO_PKG_VERSION"),
        config.data_dir.display(),
        sidecar_dir.display(),
        control.address,
        gateway
    );
    record(
        config,
        &snapshot("starting", &running, &control.address, &gateway),
    );

    let parent_gone = if config.parent_stdin {
        Some(watch_parent_stdin())
    } else {
        None
    };
    let timeout = Duration::from_millis(config.health_timeout_ms);
    let outcome = match wait_for_health(&spawner, &mut running, parent_gone.as_ref(), timeout) {
        Ok(outcome) => outcome,
        Err(error) => {
            record(
                config,
                &snapshot("failed", &running, &control.address, &gateway),
            );
            stop(
                &mut running,
                &control,
                Duration::from_millis(config.shutdown_grace_ms),
            );
            return Err(error);
        }
    };
    if outcome == Outcome::ParentGone {
        record(
            config,
            &snapshot("stopped", &running, &control.address, &gateway),
        );
        stop(
            &mut running,
            &control,
            Duration::from_millis(config.shutdown_grace_ms),
        );
        return Ok(());
    }
    record(
        config,
        &snapshot("ready", &running, &control.address, &gateway),
    );

    let mut failure: Option<String> = None;
    loop {
        if parent_gone
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
        {
            break;
        }
        match poll_children(config, &spawner, &mut running, &control, &gateway) {
            Ok(()) => {}
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
        thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }

    let final_state = if failure.is_some() {
        "failed"
    } else {
        "stopped"
    };
    record(
        config,
        &snapshot(final_state, &running, &control.address, &gateway),
    );
    stop(
        &mut running,
        &control,
        Duration::from_millis(config.shutdown_grace_ms),
    );
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Ready,
    ParentGone,
}

fn wait_for_health(
    spawner: &Spawner,
    running: &mut [Running],
    parent_gone: Option<&Arc<AtomicBool>>,
    timeout: Duration,
) -> Result<Outcome, String> {
    let deadline = Instant::now() + timeout;
    loop {
        if parent_gone.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
            return Ok(Outcome::ParentGone);
        }
        poll_children_once(spawner, running)?;
        if all_healthy(running) {
            return Ok(Outcome::Ready);
        }
        if Instant::now() >= deadline {
            let waiting: Vec<_> = running
                .iter()
                .filter(|service| service.spec.has_http())
                .filter(|service| !service.is_healthy())
                .map(|service| format!("{}:{}", service.spec.name, service.port.unwrap_or(0)))
                .collect();
            return Err(format!(
                "the desktop profile did not become healthy within {} ms; still waiting on {}",
                timeout.as_millis(),
                waiting.join(", ")
            ));
        }
        thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }
}

fn all_healthy(running: &[Running]) -> bool {
    !running.is_empty()
        && running
            .iter()
            .filter(|service| service.spec.has_http())
            .all(Running::is_healthy)
}

fn poll_children(
    config: &Config,
    spawner: &Spawner,
    running: &mut [Running],
    control: &Control,
    gateway: &str,
) -> Result<(), String> {
    if poll_children_once(spawner, running)? {
        record(
            config,
            &snapshot("ready", running, &control.address, gateway),
        );
    }
    Ok(())
}

/// Replace every exited guard, or fail when a service used its restart budget.
fn poll_children_once(spawner: &Spawner, running: &mut [Running]) -> Result<bool, String> {
    let mut changed = false;
    for service in running.iter_mut() {
        let exited = match service.child.try_wait() {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(error) => {
                return Err(format!("cannot observe {}: {error}", service.spec.name));
            }
        };
        if !exited {
            continue;
        }
        let restarts = service.restarts + 1;
        if restarts > MAX_RESTARTS {
            return Err(format!(
                "{} exited {} times; the restart budget is exhausted",
                service.spec.name, restarts
            ));
        }
        let backoff = Duration::from_millis(
            (250_u64.saturating_mul(1 << (restarts - 1).min(3))).min(MAX_BACKOFF_MS),
        );
        eprintln!(
            "qr-desktop: {} exited; restart {restarts} of {MAX_RESTARTS} in {} ms",
            service.spec.name,
            backoff.as_millis()
        );
        thread::sleep(backoff);
        let child = spawner.spawn(service.spec, &service.program)?;
        service.child = child;
        service.restarts = restarts;
        changed = true;
    }
    Ok(changed)
}

struct Running {
    spec: ServiceSpec,
    program: PathBuf,
    port: Option<u16>,
    child: Child,
    restarts: u32,
}

impl Running {
    fn is_healthy(&self) -> bool {
        let Some(port) = self.port else {
            return true;
        };
        health::probe(
            &format!("127.0.0.1:{port}"),
            Duration::from_millis(PROBE_TIMEOUT_MS),
        )
    }
}

struct Spawner {
    supervisor: PathBuf,
    data_dir: PathBuf,
    control: String,
    plan: PortPlan,
    static_dir: Option<PathBuf>,
    config_root: Option<PathBuf>,
    image_digest: String,
}

impl Spawner {
    fn spawn(&self, spec: ServiceSpec, program: &Path) -> Result<Child, String> {
        let mut command = Command::new(&self.supervisor);
        command
            .arg("--guard")
            .arg("--control")
            .arg(&self.control)
            .arg("--")
            .arg(program);
        for key in [
            "QR_BEARER_TOKEN",
            "QR_DATABASE_URL",
            "QR_QUEUE_URL",
            "QR_OBJECT_STORE_URL",
            "QR_OBJECT_BUCKET",
        ] {
            command.env_remove(key);
        }
        command.env("QR_PROFILE", "desktop");
        command.env("QR_DATA_DIR", &self.data_dir);
        command.env("QR_IMAGE_DIGEST", &self.image_digest);
        command.env("QR_DESKTOP_CONTROL", &self.control);
        if let Some(directory) = &self.config_root {
            command.env("QR_CONFIG_ROOT", directory);
        }
        let url = |port: u16| format!("http://127.0.0.1:{port}");
        if let Some(port) = self.plan.port_for(spec.name) {
            command.env("QR_BIND", format!("127.0.0.1:{port}"));
        }
        match spec.kind {
            Kind::Gateway => {
                command
                    .env("QR_DATA_URL", url(self.plan.data))
                    .env("QR_MODEL_URL", url(self.plan.model))
                    .env("QR_EVIDENCE_URL", url(self.plan.evidence))
                    .env("QR_SIMULATION_URL", url(self.plan.simulation));
                if let Some(directory) = &self.static_dir {
                    command.env("QR_STATIC_DIR", directory);
                }
            }
            Kind::Http if matches!(spec.name, "qr-model" | "qr-evidence") => {
                command.env("QR_DATA_URL", url(self.plan.data));
            }
            Kind::Worker => {
                command
                    .env("QR_DATA_URL", url(self.plan.data))
                    .env("QR_MODEL_URL", url(self.plan.model))
                    .env("QR_EVIDENCE_URL", url(self.plan.evidence));
            }
            Kind::Http => {}
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        crate::console::hide_console(&mut command);
        command
            .spawn()
            .map_err(|error| format!("cannot start {}: {error}", spec.name))
    }
}

struct Control {
    address: String,
    listener: Arc<Mutex<Option<TcpListener>>>,
    connections: Arc<Mutex<Vec<TcpStream>>>,
    closed: Arc<AtomicBool>,
}

impl Control {
    fn bind() -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| format!("cannot bind the control socket: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("cannot read the control address: {error}"))?
            .to_string();
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("cannot configure the control socket: {error}"))?;
        let connections = Arc::new(Mutex::new(Vec::new()));
        let listener = Arc::new(Mutex::new(Some(listener)));
        let accepted = Arc::clone(&connections);
        let shared = Arc::clone(&listener);
        let closed = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&closed);
        thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let Ok(slot) = shared.lock() else {
                    break;
                };
                let Some(listener) = slot.as_ref() else {
                    break;
                };
                match listener.accept() {
                    Ok((stream, _)) => {
                        drop(slot);
                        if let Ok(mut guard) = accepted.lock() {
                            // The close path sets this flag before it clears
                            // the list, so a connection accepted in the gap is
                            // closed here instead of outliving the supervisor.
                            if !stop.load(Ordering::SeqCst) && guard.len() < 32 {
                                guard.push(stream);
                            }
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        drop(slot);
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            address,
            listener,
            connections,
            closed,
        })
    }

    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        if let Ok(mut listener) = self.listener.lock() {
            // Dropping the listener also refuses a connection still in the
            // backlog, so a guard that has not been accepted yet still sees
            // the close and ends its service.
            *listener = None;
        }
        if let Ok(mut guard) = self.connections.lock() {
            guard.clear();
        }
    }
}

fn stop(running: &mut [Running], control: &Control, grace: Duration) {
    control.close();
    let deadline = Instant::now() + grace;
    loop {
        let mut alive = false;
        for service in running.iter_mut() {
            match service.child.try_wait() {
                Ok(Some(_)) => {}
                Ok(None) => alive = true,
                Err(_) => alive = true,
            }
        }
        if !alive || Instant::now() >= deadline {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    for service in running.iter_mut() {
        if let Ok(None) = service.child.try_wait() {
            let _ = service.child.kill();
            let _ = service.child.wait();
        }
    }
}

fn watch_parent_stdin() -> Arc<AtomicBool> {
    let gone = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&gone);
    thread::spawn(move || {
        let mut buffer = [0_u8; 1];
        loop {
            match std::io::stdin().read(&mut buffer) {
                Ok(0) => break,
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        flag.store(true, Ordering::SeqCst);
    });
    gone
}

fn allocate_with_retry(config: &Config) -> Result<PortPlan, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match ports::allocate(config.gateway_port, config.internal_base) {
            Ok(plan) => return Ok(plan),
            Err(error) if Instant::now() < deadline => {
                eprintln!("qr-desktop: {error}; retrying while a previous stack exits");
                thread::sleep(Duration::from_millis(250));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn resolve_sidecar_dir(config: &Config) -> Result<PathBuf, String> {
    let gateway = format!("qr-gateway{}", std::env::consts::EXE_SUFFIX);
    if let Some(directory) = &config.sidecar_dir {
        if !directory.join(&gateway).is_file() {
            return Err(format!(
                "--sidecar-dir {} does not contain {gateway}",
                directory.display()
            ));
        }
        return Ok(directory.clone());
    }
    let mut candidates = Vec::new();
    if let Some(directory) = &config.resource_dir {
        candidates.push(directory.join("sidecars"));
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        candidates.push(parent.join("sidecars"));
        candidates.push(parent.to_path_buf());
    }
    for candidate in candidates {
        if candidate.join(&gateway).is_file() {
            return Ok(candidate);
        }
    }
    Err(
        "no sidecar directory found; the installer must bundle the six service binaries under sidecars/"
            .to_owned(),
    )
}

fn sidecar_programs(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut programs = Vec::with_capacity(SERVICES.len());
    for spec in SERVICES {
        let program = directory.join(format!("{}{}", spec.program, std::env::consts::EXE_SUFFIX));
        if !program.is_file() {
            return Err(format!(
                "missing {}; the installer must bundle the six service binaries",
                program.display()
            ));
        }
        programs.push(program);
    }
    Ok(programs)
}

fn snapshot(state: &'static str, running: &[Running], control: &str, gateway: &str) -> Status {
    Status {
        profile: "desktop",
        state,
        control: control.to_owned(),
        gateway: gateway.to_owned(),
        services: running
            .iter()
            .map(|service| ServiceStatus {
                name: service.spec.name,
                port: service.port,
                pid: service.child.id(),
                restarts: service.restarts,
            })
            .collect(),
    }
}

fn record(config: &Config, status: &Status) {
    if let Err(error) = status::write(&config.data_dir, status) {
        eprintln!("qr-desktop: cannot write the diagnostic record: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_data_directory_is_refused() {
        let error = prepare_data_dir(Path::new("")).expect_err("blank");
        assert!(error.contains("QR_DATA_DIR"), "{error}");
    }

    #[test]
    fn a_data_directory_is_created_and_made_absolute() {
        let root = std::env::temp_dir().join(format!("qr-supervisor-data-{}", std::process::id()));
        let nested = root.join("nested");
        let resolved = prepare_data_dir(&nested).expect("directory");
        assert!(resolved.is_absolute());
        assert!(resolved.ends_with("nested"));
        assert!(resolved.is_dir());
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
