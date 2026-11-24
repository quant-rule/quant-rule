//! Golden output for the v3 renderer.
//!
//! The fixture descriptor is the checked-in example or lab document with secret,
//! CA, and ingress paths rewritten to a temporary directory. The rendered YAML,
//! scripts, manifests, and public certificates are compared byte for byte. The
//! secret values and private keys are fake fixtures and are compared with their
//! source copies or checked by mode, not stored twice.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

const SAMPLE: &str = "deploy/v3/deploy.example.json";
const LAB: &str = "deploy/v3/deploy.lab.json";
const SAMPLE_EXPECTED: &str = "crates/quantrule-deploy/tests/fixtures/render-v3/expected";
const LAB_EXPECTED: &str = "crates/quantrule-deploy/tests/fixtures/render-v3-lab/expected";

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-render-golden-{}-{}",
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

/// Stage the fake stateful PKI leaves under the CA parent.
fn stage_pki(directory: &Path) -> PathBuf {
    let root = directory.join("pki");
    std::fs::create_dir_all(root.join("postgres")).expect("postgres pki");
    std::fs::create_dir_all(root.join("etcd")).expect("etcd pki");
    std::fs::create_dir_all(root.join("rabbitmq")).expect("rabbitmq pki");
    std::fs::create_dir_all(root.join("minio/CAs")).expect("minio pki");
    let ca = root.join("ca.crt");
    write_mode(
        &ca,
        "-----BEGIN CERTIFICATE-----\ngolden fixture CA\n-----END CERTIFICATE-----\n",
        0o644,
    );
    for (directory, name) in [
        ("postgres", "server"),
        ("etcd", "server"),
        ("etcd", "peer"),
        ("etcd", "client"),
        ("rabbitmq", "server"),
    ] {
        write_mode(
            &root.join(directory).join(format!("{name}.crt")),
            &format!(
                "-----BEGIN CERTIFICATE-----\n{name} fixture certificate\n-----END CERTIFICATE-----\n"
            ),
            0o644,
        );
        write_mode(
            &root.join(directory).join(format!("{name}.key")),
            &format!("{name}-fixture-private-key\n"),
            0o600,
        );
    }
    write_mode(
        &root.join("minio/public.crt"),
        "-----BEGIN CERTIFICATE-----\nminio fixture certificate\n-----END CERTIFICATE-----\n",
        0o644,
    );
    write_mode(
        &root.join("minio/private.key"),
        "minio-fixture-private-key\n",
        0o600,
    );
    write_mode(
        &root.join("minio/CAs/ca.crt"),
        "-----BEGIN CERTIFICATE-----\ngolden fixture CA\n-----END CERTIFICATE-----\n",
        0o644,
    );
    ca
}

fn local_descriptor(descriptor_path: &str, directory: &Path) -> PathBuf {
    let mut value: Value =
        serde_json::from_str(&std::fs::read_to_string(repo(descriptor_path)).expect("descriptor"))
            .expect("descriptor JSON");
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

fn render(descriptor: &Path, out: &Path) {
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
}

fn compare_golden(out: &Path, expected: &str, required: &[&str]) {
    let expected = repo(expected);
    let mut files = Vec::new();
    walk(&expected, &mut files);
    assert!(!files.is_empty());
    for path in files {
        let relative = path.strip_prefix(&expected).expect("relative");
        let actual = out.join(relative);
        assert!(actual.is_file(), "missing {}", relative.display());
        assert_eq!(
            std::fs::read(&actual).expect("actual"),
            std::fs::read(&path).expect("expected"),
            "{}",
            relative.display()
        );
    }

    for key in [
        "pki/ingress.key",
        "pki/postgres/server.key",
        "pki/etcd/server.key",
        "pki/etcd/peer.key",
        "pki/etcd/client.key",
        "pki/rabbitmq/server.key",
        "pki/minio/private.key",
    ] {
        let path = out.join(key);
        assert!(path.is_file(), "{key} is rendered");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path)
                .expect("key metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600, "{key}");
        }
    }

    for name in [
        "gateway_token",
        "gateway_database_url",
        "gateway_owner_database_url",
        "data_database_url",
        "data_owner_database_url",
        "evidence_database_url",
        "evidence_owner_database_url",
        "simulation_database_url",
        "simulation_owner_database_url",
        "queue_url",
        "object_store_access_key",
        "object_store_secret_key",
        "postgres_superuser_password",
        "patroni_replication_password",
        "patroni_rewind_password",
        "patroni_rest_password",
        "rabbitmq_erlang_cookie",
        "minio_root_user",
        "minio_root_password",
    ] {
        let source = std::fs::read_to_string(repo(&format!("deploy/v3/secrets/{name}")))
            .expect("source secret");
        let rendered =
            std::fs::read_to_string(out.join("secrets").join(name)).expect("rendered secret");
        assert_eq!(rendered, format!("{}\n", source.trim_end()), "{name}");
    }

    let manifest = std::fs::read_to_string(out.join("render.json")).expect("render manifest");
    assert!(
        manifest.contains("\"descriptor_hash\": \"sha256:"),
        "{manifest}"
    );
    for name in required {
        let name = *name;
        assert!(out.join(name).is_file(), "{name} is rendered");
        assert!(
            manifest.contains(name),
            "{name} is in render.json: {manifest}"
        );
    }
    let stateful =
        std::fs::read_to_string(out.join("stateful-manifest.json")).expect("stateful manifest");
    assert!(stateful.contains("\"scope\": \"secret\""), "{stateful}");
    let secrets_manifest =
        std::fs::read_to_string(out.join("secrets/manifest.json")).expect("secret manifest");
    assert!(
        secrets_manifest.contains("\"scope\": \"stateful\""),
        "{secrets_manifest}"
    );
    assert!(
        !stateful.contains(
            &std::fs::read_to_string(repo("deploy/v3/secrets/postgres_superuser_password"))
                .expect("superuser password")
                .trim()
                .to_owned()
        ),
        "the stateful manifest contains a password"
    );
}

#[test]
fn the_render_matches_its_checked_in_golden_files() {
    let directory = temp_dir();
    let descriptor = local_descriptor(SAMPLE, &directory);
    let out = directory.join("out");
    render(&descriptor, &out);
    compare_golden(
        &out,
        SAMPLE_EXPECTED,
        &[
            "patroni-infra-1.yml",
            "etcd-infra-1.conf.yml",
            "haproxy-edge-1.cfg",
            "rabbitmq-infra-1.conf",
            "rabbitmq-enabled_plugins",
            "minio-infra-1.conf",
            "minio-policy.json",
            "stateful-manifest.json",
            "prometheus.yml",
            "alert-rules.yml",
            "alertmanager.yml",
            "loki.yml",
            "blackbox.yml",
            "promtail-core-1.yml",
            "observability-manifest.json",
            "grafana/dashboards/overview.json",
            "grafana/dashboards/ha-matrix.json",
            "grafana/dashboards/capacity.json",
            "grafana/dashboards/runs-evidence.json",
            "grafana/provisioning/datasources/datasources.yml",
            "grafana/provisioning/dashboards/dashboards.yml",
        ],
    );
    let rabbitmq =
        std::fs::read_to_string(out.join("rabbitmq-infra-1.conf")).expect("rabbitmq conf");
    for expected in [
        "listeners.tcp = none",
        "listeners.ssl.default = 5671",
        "cluster_formation.classic_config.nodes.3 = rabbit@rabbitmq-infra-3",
        "cluster_partition_handling = pause_minority",
        "default_queue_type = quorum",
        "disk_free_limit.absolute = 5120MiB",
        "vm_memory_high_watermark.relative = 0.60",
        "prometheus.tcp.port = 15692",
    ] {
        assert!(rabbitmq.contains(expected), "{expected}: {rabbitmq}");
    }
    let plugins = std::fs::read_to_string(out.join("rabbitmq-enabled_plugins")).expect("plugins");
    assert_eq!(plugins, "[rabbitmq_prometheus].\n");
    let haproxy = std::fs::read_to_string(out.join("haproxy-edge-1.cfg")).expect("haproxy");
    assert!(haproxy.contains("bind *:5671"), "{haproxy}");
    assert!(
        haproxy.contains("server rabbitmq-infra-1 rabbitmq-infra-1:5671 check"),
        "{haproxy}"
    );
    assert!(haproxy.contains("bind *:9000"), "{haproxy}");
    assert!(
        haproxy.contains("server minio-infra-1 minio-infra-1:9000 check"),
        "{haproxy}"
    );
    let minio = std::fs::read_to_string(out.join("minio-infra-1.conf")).expect("minio conf");
    for expected in [
        "MINIO_VOLUMES=\"https://minio-infra-1:9000/data1 https://minio-infra-2:9000/data1 https://minio-infra-3:9000/data1 https://minio-infra-4:9000/data1\"",
        "MINIO_SERVER_URL=\"https://minio:9000\"",
        "MINIO_BROWSER=\"off\"",
        "MINIO_STORAGE_CLASS_STANDARD=\"EC:2\"",
        "MINIO_CONFIG_ENV_FILE=\"/etc/minio/minio.conf\"",
    ] {
        assert!(minio.contains(expected), "{expected}: {minio}");
    }
    let policy = std::fs::read_to_string(out.join("minio-policy.json")).expect("minio policy");
    assert!(policy.contains("\"s3:PutObject\""), "{policy}");
    assert!(policy.contains("\"arn:aws:s3:::quantrule/*\""), "{policy}");
    let prometheus = std::fs::read_to_string(out.join("prometheus.yml")).expect("prometheus");
    for expected in [
        "\"tasks.gateway\"",
        "\"tasks.ingress\"",
        "\"patroni-infra-1:8008\"",
        "\"haproxy-edge-1:8404\"",
        "\"rabbitmq-infra-1:15692\"",
        "\"minio-infra-1:9000\"",
        "\"node-core-1:9100\"",
        "password_file: \"/etc/prometheus/secrets/patroni_rest_password\"",
        "ca_file: \"/etc/quantrule/pki/ca.crt\"",
        "check: \"health\"",
        "check: \"ready\"",
    ] {
        assert!(prometheus.contains(expected), "{expected}: {prometheus}");
    }
    let rules = std::fs::read_to_string(out.join("alert-rules.yml")).expect("rules");
    for row in 1..=17 {
        assert!(
            rules.contains(&format!("failure_row: \"F{row:02}\"")),
            "F{row:02}"
        );
    }
    assert_eq!(rules.matches("failure_row:").count(), 38, "{rules}");
    for expected in [
        "GatewayReplicaDown",
        "EtcdNodeDown",
        "PostgresNoPrimaryBackend",
        "MinioClusterUnhealthy",
        "SwarmManagerQuorumAtRisk",
        "patroni_primary",
        "patroni_xlog_received_location - patroni_xlog_replayed_location",
        "patroni_sync_standby",
        "minio_cluster_drive_offline_total",
        "minio_cluster_health_erasure_set_healing_drives",
        "minio_cluster_health_status",
    ] {
        assert!(rules.contains(expected), "{expected}: {rules}");
    }
    assert!(!rules.contains("patroni_master"), "{rules}");
    assert!(
        !rules.contains("minio_cluster_disk_offline_total"),
        "{rules}"
    );
    assert!(!rules.contains("minio_cluster_heal_status"), "{rules}");
    let alertmanager = std::fs::read_to_string(out.join("alertmanager.yml")).expect("alertmanager");
    assert_eq!(
        alertmanager.matches("name: \"quantrule-null\"").count(),
        1,
        "{alertmanager}"
    );
    assert!(!alertmanager.contains("webhook_configs"), "{alertmanager}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_lab_render_matches_its_checked_in_golden_files() {
    let directory = temp_dir();
    let descriptor = local_descriptor(LAB, &directory);
    let out = directory.join("out");
    render(&descriptor, &out);
    compare_golden(
        &out,
        LAB_EXPECTED,
        &[
            "patroni-lab-1.yml",
            "etcd-lab-1.conf.yml",
            "haproxy-lab-1.cfg",
            "rabbitmq-lab-1.conf",
            "rabbitmq-enabled_plugins",
            "minio-lab-1.conf",
            "minio-policy.json",
            "stateful-manifest.json",
        ],
    );
    let manifest = std::fs::read_to_string(out.join("render.json")).expect("render manifest");
    assert!(
        manifest.contains("\"evidence\": \"Simulated HA\""),
        "{manifest}"
    );
    let patroni = std::fs::read_to_string(out.join("patroni-lab-1.yml")).expect("patroni");
    assert!(patroni.contains("mode: \"off\""), "{patroni}");
    assert!(!patroni.contains("/dev/watchdog"), "{patroni}");
    let minio = std::fs::read_to_string(out.join("minio-lab-1.conf")).expect("minio conf");
    assert!(minio.contains("https://minio-lab-1:9000/data2"), "{minio}");
    assert!(
        minio.contains("MINIO_STORAGE_CLASS_STANDARD=\"EC:3\""),
        "{minio}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

/// Regenerate the checked-in golden fixtures deliberately:
/// `cargo test -p quantrule-deploy --test render_golden -- --ignored`.
#[test]
#[ignore = "rewrites the checked-in golden fixtures"]
fn regenerate_the_golden_fixture() {
    for (descriptor_path, expected_path) in [(SAMPLE, SAMPLE_EXPECTED), (LAB, LAB_EXPECTED)] {
        let directory = temp_dir();
        let descriptor = local_descriptor(descriptor_path, &directory);
        let out = directory.join("out");
        render(&descriptor, &out);

        let expected = repo(expected_path);
        let mut rendered = Vec::new();
        walk(&out, &mut rendered);
        let mut keep = std::collections::BTreeSet::new();
        for path in rendered {
            let relative = path.strip_prefix(&out).expect("relative");
            if relative == Path::new("render.json")
                || relative
                    .extension()
                    .is_some_and(|extension| extension == "key")
                || (relative.starts_with("secrets")
                    && relative != Path::new("secrets/manifest.json"))
            {
                continue;
            }
            keep.insert(relative.to_path_buf());
            let destination = expected.join(relative);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).expect("expected parent");
            }
            std::fs::copy(&path, &destination).expect("copy golden");
        }
        let mut existing = Vec::new();
        walk(&expected, &mut existing);
        for path in existing {
            let relative = path.strip_prefix(&expected).expect("relative");
            if !keep.contains(relative) {
                std::fs::remove_file(&path).expect("remove stale golden");
            }
        }
        let _ = std::fs::remove_dir_all(directory);
    }
}

fn walk(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            walk(&path, files);
        } else {
            files.push(path);
        }
    }
}
