//! Fake-CLI tests for `tools/swarm.sh`.
//!
//! The wrapper is checked in, so its argument forwarding, snapshot-before-
//! upgrade, rollback target, and destructive-command refusal are tested with a
//! fake `qr-deploy` and a fake `docker`. The fast tier stays Docker-free.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-swarm-wrapper-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn write_mode(path: &Path, content: &str, mode: u32) {
    std::fs::write(path, content).expect("write file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("mode");
    }
    #[cfg(not(unix))]
    let _ = mode;
}

struct Fixture {
    directory: PathBuf,
    bin: PathBuf,
    out: PathBuf,
    state: PathBuf,
    qr_log: PathBuf,
    docker_log: PathBuf,
    bootstrap_log: PathBuf,
}

fn fixture() -> Fixture {
    let directory = temp_dir();
    let bin = directory.join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    write_mode(
        &bin.join("qr-deploy"),
        r#"#!/bin/sh
set -eu
printf 'qr-deploy %s\n' "$*" >> "$FAKE_QR_LOG"
if [ "$1" = "render" ]; then
    out=""
    previous=""
    for argument in "$@"; do
        if [ "$previous" = "--out" ]; then out=$argument; fi
        previous=$argument
    done
    [ -n "$out" ] || exit 1
    mkdir -p "$out"
    [ -f "$out/bootstrap.sh" ] || printf '#!/bin/sh\nprintf "bootstrap\\n" >> "$FAKE_BOOTSTRAP_LOG"\n' > "$out/bootstrap.sh"
    printf 'new stack\n' > "$out/stack.yaml"
    printf '{"stack":"quantrule-ha"}\n' > "$out/render.json"
fi
exit 0
"#,
        0o755,
    );
    write_mode(
        &bin.join("docker"),
        r#"#!/bin/sh
set -eu
printf 'docker %s\n' "$*" >> "$FAKE_DOCKER_LOG"
case "$1" in
    info) exit 0 ;;
    stack)
        case "$2" in
            services) printf 'quantrule-ha_gateway 2/2 image\nquantrule-ha_ingress 2/2 traefik\n' ;;
            deploy) exit 0 ;;
            *) exit 1 ;;
        esac
        ;;
    node)
        case "$2" in
            ls) printf 'n01 node-01 Ready Active Leader\n' ;;
            *) exit 1 ;;
        esac
        ;;
    *) exit 1 ;;
esac
"#,
        0o755,
    );
    let out = directory.join("out");
    std::fs::create_dir_all(&out).expect("out");
    write_mode(
        &out.join("bootstrap.sh"),
        "#!/bin/sh\nprintf 'bootstrap\\n' >> \"$FAKE_BOOTSTRAP_LOG\"\n",
        0o755,
    );
    std::fs::write(out.join("stack.yaml"), "old stack\n").expect("old stack");
    std::fs::write(out.join("render.json"), "{\"stack\":\"quantrule-ha\"}\n").expect("manifest");
    let state = directory.join("state");
    let qr_log = directory.join("qr.log");
    let docker_log = directory.join("docker.log");
    let bootstrap_log = directory.join("bootstrap.log");
    Fixture {
        directory,
        bin,
        out,
        state,
        qr_log,
        docker_log,
        bootstrap_log,
    }
}

impl Fixture {
    fn run(&self, args: &[&str]) -> Output {
        Command::new("bash")
            .arg(repo("tools/swarm.sh"))
            .args(args)
            .env(
                "PATH",
                format!("{}:/usr/local/bin:/usr/bin:/bin", self.bin.display()),
            )
            .env("QR_DEPLOY_BIN", self.bin.join("qr-deploy"))
            .env("DESCRIPTOR", "deploy/v3/deploy.example.json")
            .env("OUT", &self.out)
            .env("QR_SWARM_STATE", &self.state)
            .env("FAKE_QR_LOG", &self.qr_log)
            .env("FAKE_DOCKER_LOG", &self.docker_log)
            .env("FAKE_BOOTSTRAP_LOG", &self.bootstrap_log)
            .output()
            .expect("run swarm wrapper")
    }

    fn qr(&self) -> String {
        std::fs::read_to_string(&self.qr_log).unwrap_or_default()
    }

    fn docker(&self) -> String {
        std::fs::read_to_string(&self.docker_log).unwrap_or_default()
    }

    fn bootstrap(&self) -> String {
        std::fs::read_to_string(&self.bootstrap_log).unwrap_or_default()
    }
}

#[test]
fn the_wrapper_is_valid_bash() {
    let output = Command::new("bash")
        .args(["-n", repo("tools/swarm.sh").to_str().expect("path")])
        .output()
        .expect("bash -n");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_and_render_forward_the_descriptor_out_and_target() {
    let fixture = fixture();
    let output = fixture.run(&["validate"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fixture
            .qr()
            .contains("validate deploy/v3/deploy.example.json --files"),
        "{}",
        fixture.qr()
    );
    let output = fixture.run(&["render"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fixture.qr().contains(&format!(
            "render deploy/v3/deploy.example.json --out {} --target all",
            fixture.out.display()
        )),
        "{}",
        fixture.qr()
    );
    let _ = std::fs::remove_dir_all(fixture.directory);
}

#[test]
fn bootstrap_renders_before_it_runs_the_generated_script() {
    let fixture = fixture();
    let output = fixture.run(&["bootstrap"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fixture.qr().contains("render "), "{}", fixture.qr());
    assert_eq!(
        fixture.bootstrap().lines().count(),
        1,
        "{}",
        fixture.bootstrap()
    );
    let _ = std::fs::remove_dir_all(fixture.directory);
}

#[test]
fn upgrade_snapshots_the_previous_stack_before_rendering_the_new_one() {
    let fixture = fixture();
    let output = fixture.run(&["upgrade"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let latest = std::fs::read_to_string(fixture.state.join("latest")).expect("latest");
    let snapshot = PathBuf::from(latest.trim());
    assert!(snapshot.join("stack.yaml").is_file());
    assert!(snapshot.join("render.json").is_file());
    assert!(snapshot.join("services.txt").is_file());
    assert_eq!(
        std::fs::read_to_string(snapshot.join("stack-name")).expect("snapshot stack name"),
        "quantrule-ha\n"
    );
    assert_eq!(
        std::fs::read_to_string(snapshot.join("stack.yaml")).expect("snapshot stack"),
        "old stack\n"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.out.join("stack.yaml")).expect("new stack"),
        "new stack\n"
    );
    assert_eq!(
        fixture.bootstrap().lines().count(),
        1,
        "{}",
        fixture.bootstrap()
    );
    assert!(
        !fixture.docker().contains("stack rm"),
        "{}",
        fixture.docker()
    );
    assert!(
        !fixture.docker().contains("secret rm"),
        "{}",
        fixture.docker()
    );
    assert!(
        !fixture.docker().contains("volume rm"),
        "{}",
        fixture.docker()
    );
    let _ = std::fs::remove_dir_all(fixture.directory);
}

#[test]
fn rollback_redeploys_the_last_snapshot_without_removing_anything() {
    let fixture = fixture();
    let output = fixture.run(&["upgrade"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let latest = std::fs::read_to_string(fixture.state.join("latest")).expect("latest");
    let snapshot = latest.trim().to_owned();
    let output = fixture.run(&["rollback"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fixture.docker().contains(&format!(
            "stack deploy --with-registry-auth -c {snapshot}/stack.yaml quantrule-ha"
        )),
        "{}",
        fixture.docker()
    );
    assert!(
        !fixture.docker().contains("stack rm"),
        "{}",
        fixture.docker()
    );
    assert!(
        !fixture.docker().contains("secret rm"),
        "{}",
        fixture.docker()
    );
    assert!(
        !fixture.docker().contains("volume rm"),
        "{}",
        fixture.docker()
    );
    let _ = std::fs::remove_dir_all(fixture.directory);
}

#[test]
fn status_and_verify_are_operator_reads() {
    let fixture = fixture();
    let output = fixture.run(&["status"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(printed.contains("quantrule-ha_gateway"), "{printed}");
    assert!(printed.contains("n01"), "{printed}");

    let output = fixture.run(&["verify"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fixture.qr().contains("check deploy/v3/deploy.example.json"),
        "{}",
        fixture.qr()
    );
    let _ = std::fs::remove_dir_all(fixture.directory);
}

#[test]
fn verify_forwards_the_remote_and_overlay_flags() {
    let fixture = fixture();
    let output = fixture.run(&["verify", "--remote", "docker", "--overlay"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fixture
            .qr()
            .contains("check deploy/v3/deploy.example.json --remote docker --overlay"),
        "{}",
        fixture.qr()
    );
    let _ = std::fs::remove_dir_all(fixture.directory);
}

#[test]
fn an_unknown_command_is_a_usage_error() {
    let fixture = fixture();
    let output = fixture.run(&["explode"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown command"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_dir_all(fixture.directory);
}
