//! Fake-Docker tests for the rendered Swarm bootstrap.
//!
//! The fast tier cannot start a Swarm. These tests render the real
//! `bootstrap.sh` with the checked-in example, put a small `docker` shim on
//! `PATH`, and assert the operator contract: object drift is refused, nodes
//! resolve by label then hostname then unique address, ambiguous/drained/foreign
//! nodes stop the run, the database jobs run before the stack, and no secret
//! value is printed.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-swarm-bootstrap-{}-{}",
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

fn stage_pki(directory: &Path) -> PathBuf {
    let root = directory.join("pki");
    std::fs::create_dir_all(root.join("postgres")).expect("postgres pki");
    std::fs::create_dir_all(root.join("etcd")).expect("etcd pki");
    std::fs::create_dir_all(root.join("rabbitmq")).expect("rabbitmq pki");
    std::fs::create_dir_all(root.join("minio/CAs")).expect("minio pki");
    let ca = root.join("ca.crt");
    write_mode(&ca, "fixture CA\n", 0o644);
    for (directory, name) in [
        ("postgres", "server"),
        ("etcd", "server"),
        ("etcd", "peer"),
        ("etcd", "client"),
        ("rabbitmq", "server"),
    ] {
        write_mode(
            &root.join(directory).join(format!("{name}.crt")),
            "fixture certificate\n",
            0o644,
        );
        write_mode(
            &root.join(directory).join(format!("{name}.key")),
            "fixture key\n",
            0o600,
        );
    }
    write_mode(
        &root.join("minio/public.crt"),
        "fixture certificate\n",
        0o644,
    );
    write_mode(&root.join("minio/private.key"), "fixture key\n", 0o600);
    write_mode(&root.join("minio/CAs/ca.crt"), "fixture CA\n", 0o644);
    ca
}

fn local_descriptor(directory: &Path) -> PathBuf {
    let mut value: Value = serde_json::from_str(
        &std::fs::read_to_string(repo("deploy/v3/deploy.example.json")).expect("v3 example"),
    )
    .expect("v3 JSON");
    let files = value["secrets"]["files"].as_object_mut().expect("files");
    for (name, path) in files.iter_mut() {
        let content = std::fs::read_to_string(repo(&format!("deploy/v3/secrets/{name}")))
            .expect("fixture secret");
        let destination = directory.join(name);
        write_mode(&destination, &content, 0o600);
        *path = serde_json::json!(destination.display().to_string());
    }
    let ca = stage_pki(directory);
    for endpoint in ["database", "queue", "object_store"] {
        value["endpoints"][endpoint]["tls_ca"] = serde_json::json!(ca.display().to_string());
    }
    let certificate = directory.join("ingress.crt");
    let key = directory.join("ingress.key");
    write_mode(
        &certificate,
        &std::fs::read_to_string(repo("deploy/v3/pki/ingress.crt")).expect("fixture cert"),
        0o644,
    );
    write_mode(
        &key,
        &std::fs::read_to_string(repo("deploy/v3/pki/ingress-key.pem")).expect("fixture key"),
        0o600,
    );
    value["ingress"]["files"] = serde_json::json!({
        "certificate": certificate.display().to_string(),
        "key": key.display().to_string(),
    });
    let path = directory.join("deploy.json");
    std::fs::write(&path, value.to_string()).expect("descriptor");
    path
}

fn render(directory: &Path) -> PathBuf {
    let descriptor = local_descriptor(directory);
    let out = directory.join("out");
    let output = Command::new(env!("CARGO_BIN_EXE_qr-deploy"))
        .args([
            "render",
            descriptor.to_str().expect("path"),
            "--out",
            out.to_str().expect("path"),
            "--target",
            "all",
        ])
        .output()
        .expect("render");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    out
}

fn fake_docker_bin(directory: &Path) -> PathBuf {
    let bin = directory.join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    let script = r#"#!/bin/sh
set -eu
state=${FAKE_STATE:?}
log=${FAKE_LOG:?}
printf '%s\n' "$*" >> "$log"

node_line() {
    awk -F'|' -v id="$1" '$1 == id { print; exit }' "$state/nodes.tsv"
}

node_labels() {
    node_line "$1" | cut -d'|' -f6
}

node_set_labels() {
    awk -F'|' -v id="$1" -v labels="$2" 'BEGIN { OFS="|" } $1 == id { $6 = labels } { print }' \
        "$state/nodes.tsv" > "$state/nodes.tsv.new"
    mv "$state/nodes.tsv.new" "$state/nodes.tsv"
}

label_has() {
    case ",$1," in
        *",$2,"*) return 0 ;;
        *) return 1 ;;
    esac
}

label_key() {
    printf '%s' "$1" | sed -n 's/.*"\([^"]*\)".*/\1/p'
}

format_value() {
    previous=""
    for argument in "$@"; do
        if [ "$previous" = "--format" ]; then
            printf '%s' "$argument"
            return 0
        fi
        previous=$argument
    done
}

case "${1:-}" in
    info)
        exit 0
        ;;
    node)
        case "${2:-}" in
            ls)
                shift 2
                quiet=0
                filter=""
                while [ "$#" -gt 0 ]; do
                    case "$1" in
                        -q) quiet=1 ;;
                        --filter) filter=$2; shift ;;
                    esac
                    shift
                done
                filter=${filter#label=}
                while IFS='|' read -r id hostname address role availability labels; do
                    [ -n "$id" ] || continue
                    if [ -n "$filter" ]; then
                        label_has "$labels" "$filter" || continue
                    fi
                    if [ "$quiet" = "1" ]; then
                        printf '%s\n' "$id"
                    else
                        printf '%s %s\n' "$id" "$hostname"
                    fi
                done < "$state/nodes.tsv"
                ;;
            inspect)
                if [ "${3:-}" = "--format" ]; then
                    format=$4
                    node=$5
                else
                    format=""
                    node=$3
                fi
                line=$(node_line "$node")
                [ -n "$line" ] || exit 1
                hostname=$(printf '%s' "$line" | cut -d'|' -f2)
                address=$(printf '%s' "$line" | cut -d'|' -f3)
                role=$(printf '%s' "$line" | cut -d'|' -f4)
                availability=$(printf '%s' "$line" | cut -d'|' -f5)
                labels=$(printf '%s' "$line" | cut -d'|' -f6)
                if [ -z "$format" ]; then
                    printf '%s\n' "$line"
                    exit 0
                fi
                case "$format" in
                    *Status.Addr*) printf '%s\n' "$address" ;;
                    *Description.Hostname*) printf '%s\n' "$hostname" ;;
                    *Spec.Availability*) printf '%s\n' "$availability" ;;
                    *Spec.Role*) printf '%s\n' "$role" ;;
                    *index\ .Spec.Labels*)
                        key=$(label_key "$format")
                        value=""
                        old_ifs=$IFS
                        IFS=,
                        for pair in $labels; do
                            case "$pair" in
                                "$key="*) value=${pair#*=} ;;
                            esac
                        done
                        IFS=$old_ifs
                        if [ -n "$value" ]; then
                            printf '%s\n' "$value"
                        else
                            printf '<no value>\n'
                        fi
                        ;;
                    *) printf '\n' ;;
                esac
                ;;
            update)
                shift 2
                add=""
                remove=""
                node=""
                while [ "$#" -gt 0 ]; do
                    case "$1" in
                        --label-add) add="$add,$2"; shift ;;
                        --label-rm) remove="$remove,$2"; shift ;;
                        *) node=$1 ;;
                    esac
                    shift
                done
                labels=$(node_labels "$node")
                old_ifs=$IFS
                IFS=,
                for key in $remove; do
                    [ -n "$key" ] || continue
                    kept=""
                    for pair in $labels; do
                        case "$pair" in
                            "$key="*) ;;
                            *) kept="$kept,$pair" ;;
                        esac
                    done
                    labels=${kept#,}
                done
                for pair in $add; do
                    [ -n "$pair" ] || continue
                    key=${pair%%=*}
                    kept=""
                    for existing in $labels; do
                        case "$existing" in
                            "$key="*) ;;
                            *) kept="$kept,$existing" ;;
                        esac
                    done
                    labels=${kept#,}
                    if [ -n "$labels" ]; then
                        labels="$labels,$pair"
                    else
                        labels=$pair
                    fi
                done
                IFS=$old_ifs
                node_set_labels "$node" "$labels"
                ;;
            *)
                exit 1
                ;;
        esac
        ;;
    network)
        case "${2:-}" in
            inspect)
                [ -f "$state/network" ] || exit 1
                format=$(format_value "$@")
                case "$format" in
                    *Driver*) printf 'overlay\n' ;;
                    *Options*) printf '{"encrypted":"true"}\n' ;;
                    *Attachable*) printf 'true\n' ;;
                    *) printf 'overlay\n' ;;
                esac
                ;;
            create)
                touch "$state/network"
                ;;
            *)
                exit 1
                ;;
        esac
        ;;
    secret|config)
        kind=$1
        file="$state/${kind}s.tsv"
        case "${2:-}" in
            inspect)
                if [ "${3:-}" = "--format" ]; then
                    name=$5
                    hash=$(awk -F'|' -v n="$name" '$1 == n { print $2; exit }' "$file")
                    [ -n "$hash" ] || exit 1
                    printf '{"quantrule.source_sha256":"%s"}\n' "$hash"
                else
                    name=$3
                    row=$(awk -F'|' -v n="$name" '$1 == n { print; exit }' "$file")
                    [ -n "$row" ] || exit 1
                    printf '%s\n' "$row"
                fi
                ;;
            create)
                label=${4#quantrule.source_sha256=}
                printf '%s|%s\n' "$5" "$label" >> "$file"
                ;;
            *)
                exit 1
                ;;
        esac
        ;;
    stack)
        case "${2:-}" in
            deploy)
                touch "$state/deployed"
                ;;
            services)
                [ -f "$state/deployed" ] || exit 1
                printf '%s\n' "${FAKE_ROWS:?}"
                ;;
            *)
                exit 1
                ;;
        esac
        ;;
    service)
        case "${2:-}" in
            inspect)
                service=$5
                case "$service" in
                    *_ingress)
                        printf '[{"Protocol":"tcp","TargetPort":443,"PublishedPort":443,"PublishMode":"host"}]\n'
                        ;;
                    *)
                        printf 'null\n'
                        ;;
                esac
                ;;
            ps)
                printf 'Running 1 second ago\n'
                ;;
            *)
                exit 1
                ;;
        esac
        ;;
    run)
        exit 0
        ;;
    *)
        exit 1
        ;;
esac
"#;
    let path = bin.join("docker");
    write_mode(&path, script, 0o755);
    path
}

fn node_table() -> String {
    [
        "n01|node-01|10.0.1.31|worker|active|",
        "n02|node-02|10.0.1.32|worker|active|",
        "n03|node-03|10.0.1.33|manager|active|",
        "n04|node-04|10.0.1.21|worker|active|",
        "n05|node-05|10.0.1.22|manager|active|",
        "n06|node-06|10.0.1.11|manager|active|",
        "n07|node-07|10.0.1.12|worker|active|",
        "n08|node-08|10.0.1.41|worker|active|",
        "n09|node-09|10.0.1.42|worker|active|",
        "n10|node-10|10.0.1.43|worker|active|",
        "n11|node-11|10.0.1.44|worker|active|",
    ]
    .join("\n")
        + "\n"
}

fn fake_state(directory: &Path, nodes: &str) -> PathBuf {
    let state = directory.join("state");
    std::fs::create_dir_all(&state).expect("state");
    std::fs::write(state.join("nodes.tsv"), nodes).expect("nodes");
    std::fs::write(state.join("secrets.tsv"), "").expect("secrets");
    std::fs::write(state.join("configs.tsv"), "").expect("configs");
    state
}

fn stack_rows() -> String {
    let digest = "sha256:cdace4a996c06cd6142fb63eb23a52136c071d9450fe0e07f5556fc26b9edabe";
    let image = format!("ghcr.io/quant-rule/quantrule-platform@{digest}");
    let mut rows = [
        ("gateway", 2),
        ("data", 2),
        ("model", 3),
        ("strategy", 3),
        ("backtest", 3),
        ("evidence", 2),
        ("simulation", 2),
    ]
    .iter()
    .map(|(name, replicas)| format!("quantrule-ha_{name} {replicas}/{replicas} {image}"))
    .collect::<Vec<_>>();
    rows.push(
        "quantrule-ha_ingress 2/2 traefik@sha256:8330e8d28853641532609f63676dc2eec8d19e2bc7fadd5505a1d0c8c254d042"
            .to_owned(),
    );
    rows.join("\n") + "\n"
}

fn run_bootstrap(out: &Path, bin: &Path, state: &Path, log: &Path) -> Output {
    run_bootstrap_with(out, bin, state, log, &[])
}

fn run_bootstrap_with(out: &Path, bin: &Path, state: &Path, log: &Path, args: &[&str]) -> Output {
    Command::new("sh")
        .arg("bootstrap.sh")
        .args(args)
        .current_dir(out)
        .env(
            "PATH",
            format!("{}:/usr/local/bin:/usr/bin:/bin", bin.display()),
        )
        .env("FAKE_STATE", state)
        .env("FAKE_LOG", log)
        .env("FAKE_ROWS", stack_rows())
        .output()
        .expect("run bootstrap")
}

fn log_text(log: &Path) -> String {
    std::fs::read_to_string(log).unwrap_or_default()
}

fn index_of(lines: &[&str], needle: &str) -> usize {
    lines
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("missing {needle} in {lines:#?}"))
}

#[test]
fn the_rendered_scripts_are_valid_shell() {
    let directory = temp_dir();
    let out = render(&directory);
    for script in [
        "bootstrap.sh",
        "jobs/cluster-bootstrap.sh",
        "jobs/db-bootstrap.sh",
        "jobs/migrate.sh",
    ] {
        let output = Command::new("sh")
            .args(["-n", out.join(script).to_str().expect("path")])
            .output()
            .expect("sh -n");
        assert!(
            output.status.success(),
            "{script}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn bootstrap_labels_nodes_migrates_before_deploying_and_never_prints_a_secret() {
    let directory = temp_dir();
    let out = render(&directory);
    let bin = fake_docker_bin(&directory);
    let state = fake_state(&directory, &node_table());
    let log = directory.join("docker.log");
    let output = run_bootstrap(&out, bin.parent().expect("bin"), &state, &log);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = log_text(&log);
    let lines: Vec<&str> = text.lines().collect();
    let cluster = index_of(&lines, "-e PGHOST=postgres");
    let bootstrap = index_of(&lines, "psql -v ON_ERROR_STOP=1 -f -");
    let migrate = index_of(&lines, "qr-gateway --migrate-only");
    let deploy = index_of(&lines, "stack deploy");
    assert!(cluster < bootstrap, "{text}");
    assert!(bootstrap < migrate, "{text}");
    assert!(migrate < deploy, "{text}");
    assert!(index_of(&lines, "secret create") < deploy, "{text}");
    assert!(text.contains("quantrule.deploy=quantrule-ha"), "{text}");
    assert!(text.contains("quantrule.host=edge-1"), "{text}");
    assert!(text.contains("quantrule.ingress=true"), "{text}");
    assert!(text.contains("quantrule.gateway=true"), "{text}");
    assert!(text.contains("--label quantrule.source_sha256="), "{text}");
    assert!(!text.contains("stack rm"), "{text}");
    assert!(!text.contains("secret rm"), "{text}");
    assert!(!text.contains("volume rm"), "{text}");

    let token = std::fs::read_to_string(repo("deploy/v3/secrets/gateway_token"))
        .expect("token")
        .trim()
        .to_owned();
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!printed.contains(&token), "the bootstrap printed the token");
    assert!(!text.contains(&token), "docker received the token");

    let secrets = std::fs::read_to_string(state.join("secrets.tsv")).expect("secrets");
    assert_eq!(secrets.lines().count(), 13, "{secrets}");
    let configs = std::fs::read_to_string(state.join("configs.tsv")).expect("configs");
    assert_eq!(configs.lines().count(), 6, "{configs}");
    for name in [
        "quantrule_traefik",
        "quantrule_traefik_dynamic",
        "quantrule_ingress_cert",
    ] {
        assert!(
            configs
                .lines()
                .any(|line| line.starts_with(&format!("{name}|"))),
            "{name}: {configs}"
        );
    }
    assert!(
        configs
            .lines()
            .all(|line| line.contains('|') && !line.contains("external")),
        "{configs}"
    );
    let config_create = index_of(&lines, "config create");
    assert!(config_create < deploy, "{text}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn network_only_exits_before_objects_labels_jobs_or_stack() {
    let directory = temp_dir();
    let out = render(&directory);
    let bin = fake_docker_bin(&directory);
    let state = fake_state(&directory, &node_table());
    let log = directory.join("docker.log");
    let first = run_bootstrap_with(
        &out,
        bin.parent().expect("bin"),
        &state,
        &log,
        &["--network-only"],
    );
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let text = String::from_utf8_lossy(&first.stdout);
    assert!(text.contains("overlay quantrule ready"), "{text}");
    let logged = log_text(&log);
    assert!(logged.contains("network create"), "{logged}");
    for forbidden in [
        "secret create",
        "config create",
        "node update",
        "run --rm",
        "stack deploy",
    ] {
        assert!(!logged.contains(forbidden), "{forbidden}: {logged}");
    }

    let second = run_bootstrap_with(
        &out,
        bin.parent().expect("bin"),
        &state,
        &log,
        &["--network-only"],
    );
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(
        log_text(&log).matches("network create").count(),
        1,
        "a rerun recreated the overlay"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_second_bootstrap_is_idempotent() {
    let directory = temp_dir();
    let out = render(&directory);
    let bin = fake_docker_bin(&directory);
    let state = fake_state(&directory, &node_table());
    let log = directory.join("docker.log");
    let first = run_bootstrap(&out, bin.parent().expect("bin"), &state, &log);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first_creates = log_text(&log).matches("secret create").count();
    let first_configs = log_text(&log).matches("config create").count();
    let second = run_bootstrap(&out, bin.parent().expect("bin"), &state, &log);
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second_creates = log_text(&log).matches("secret create").count();
    let second_configs = log_text(&log).matches("config create").count();
    assert_eq!(first_creates, 13);
    assert_eq!(first_configs, 6);
    assert_eq!(second_creates, first_creates, "a rerun recreated an object");
    assert_eq!(second_configs, first_configs, "a rerun recreated a config");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_changed_secret_file_is_refused_as_drift() {
    let directory = temp_dir();
    let out = render(&directory);
    let bin = fake_docker_bin(&directory);
    let state = fake_state(&directory, &node_table());
    let log = directory.join("docker.log");
    let first = run_bootstrap(&out, bin.parent().expect("bin"), &state, &log);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    std::fs::OpenOptions::new()
        .append(true)
        .open(out.join("secrets/gateway_token"))
        .expect("token")
        .write_all(b"rotated\n")
        .expect("append");
    let second = run_bootstrap(&out, bin.parent().expect("bin"), &state, &log);
    assert_eq!(second.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("already exists with a different rendered file"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_changed_config_file_is_refused_as_drift() {
    let directory = temp_dir();
    let out = render(&directory);
    let bin = fake_docker_bin(&directory);
    let state = fake_state(&directory, &node_table());
    let log = directory.join("docker.log");
    let first = run_bootstrap(&out, bin.parent().expect("bin"), &state, &log);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    std::fs::OpenOptions::new()
        .append(true)
        .open(out.join("traefik-dynamic.yml"))
        .expect("dynamic")
        .write_all(b"# changed\n")
        .expect("append");
    let second = run_bootstrap(&out, bin.parent().expect("bin"), &state, &log);
    assert_eq!(second.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("already exists with a different rendered file"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_foreign_deployment_label_is_refused() {
    let directory = temp_dir();
    let out = render(&directory);
    let bin = fake_docker_bin(&directory);
    let nodes = node_table().replace(
        "n06|node-06|10.0.1.11|manager|active|",
        "n06|node-06|10.0.1.11|manager|active|quantrule.deploy=other",
    );
    let state = fake_state(&directory, &nodes);
    let log = directory.join("docker.log");
    let output = run_bootstrap(&out, bin.parent().expect("bin"), &state, &log);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("belongs to deployment other"), "{stderr}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_ambiguous_host_label_is_refused() {
    let directory = temp_dir();
    let out = render(&directory);
    let bin = fake_docker_bin(&directory);
    let nodes = node_table()
        .replace(
            "n01|node-01|10.0.1.31|worker|active|",
            "n01|node-01|10.0.1.31|worker|active|quantrule.host=compute-1",
        )
        .replace(
            "n02|node-02|10.0.1.32|worker|active|",
            "n02|node-02|10.0.1.32|worker|active|quantrule.host=compute-1",
        );
    let state = fake_state(&directory, &nodes);
    let log = directory.join("docker.log");
    let output = run_bootstrap(&out, bin.parent().expect("bin"), &state, &log);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("matches more than one node by quantrule.host"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_drained_node_is_refused() {
    let directory = temp_dir();
    let out = render(&directory);
    let bin = fake_docker_bin(&directory);
    let nodes = node_table().replace(
        "n01|node-01|10.0.1.31|worker|active|",
        "n01|node-01|10.0.1.31|worker|drain|",
    );
    let state = fake_state(&directory, &nodes);
    let log = directory.join("docker.log");
    let output = run_bootstrap(&out, bin.parent().expect("bin"), &state, &log);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("activate and undrain"), "{stderr}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_unmapped_host_is_refused() {
    let directory = temp_dir();
    let out = render(&directory);
    let bin = fake_docker_bin(&directory);
    let nodes = node_table()
        .lines()
        .filter(|line| !line.starts_with("n01|"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let state = fake_state(&directory, &nodes);
    let log = directory.join("docker.log");
    let output = run_bootstrap(&out, bin.parent().expect("bin"), &state, &log);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot map host compute-1"), "{stderr}");
    let _ = std::fs::remove_dir_all(directory);
}
