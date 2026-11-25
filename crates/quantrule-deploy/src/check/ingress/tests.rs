use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn descriptor() -> Descriptor {
    serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.example.json"))
        .expect("v3 example")
}

struct FakeCertificate {
    facts: Result<CertificateFacts, String>,
}

impl CertificateQuery for FakeCertificate {
    fn inspect(&self, _path: &Path) -> Result<CertificateFacts, String> {
        self.facts.clone()
    }
}

fn healthy_certificate() -> FakeCertificate {
    FakeCertificate {
        facts: Ok(CertificateFacts {
            dns_names: vec!["quantrule.dev".to_owned()],
            valid_for_days: true,
        }),
    }
}

fn cert_files() -> (std::path::PathBuf, std::path::PathBuf) {
    let ticket = COUNTER.fetch_add(1, Ordering::Relaxed);
    let certificate = temp_path(&format!("cert-{ticket}.pem"));
    let key = temp_path(&format!("key-{ticket}.pem"));
    std::fs::write(&certificate, "fixture certificate\n").expect("cert");
    std::fs::write(&key, "fixture key\n").expect("key");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&certificate, std::fs::Permissions::from_mode(0o644))
            .expect("cert mode");
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).expect("key mode");
    }
    (certificate, key)
}

fn descriptor_with_files(certificate: &Path, key: &Path) -> Descriptor {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.example.json"))
            .expect("example");
    value["ingress"]["files"] = serde_json::json!({
        "certificate": certificate.display().to_string(),
        "key": key.display().to_string(),
    });
    serde_json::from_value(value).expect("descriptor")
}

#[test]
fn the_static_rules_accept_the_checked_in_file_and_refuse_port_80() {
    assert!(static_issues(ingress::STATIC_CONFIG).is_empty());
    let broken = ingress::STATIC_CONFIG.replace("address: \":443\"", "address: \":80\"");
    let issues = static_issues(&broken);
    assert!(
        issues.iter().any(|issue| issue.contains("443")),
        "{issues:?}"
    );
    assert!(
        issues.iter().any(|issue| issue.contains(":80")),
        "{issues:?}"
    );
}

#[test]
fn the_dynamic_rules_accept_the_rendered_file_and_refuse_basic_auth() {
    let descriptor = descriptor();
    let text = ingress::dynamic_yaml(&descriptor);
    assert!(dynamic_issues(&text, &descriptor).is_empty());
    let broken = text.replace("security-headers", "basicAuth");
    assert!(
        dynamic_issues(&broken, &descriptor)
            .iter()
            .any(|issue| issue.contains("authentication")),
        "{broken}"
    );
}

#[test]
fn keepalived_rows_cover_every_ingress_host_with_the_two_priorities() {
    let descriptor = descriptor();
    let rows = keepalived_rows(&descriptor);
    assert_eq!(rows.len(), descriptor.ingress.hosts.len());
    assert!(
        rows.iter().all(|row| !row.is_failure()),
        "{}",
        render(&rows)
    );
    assert!(rows[0].detail.as_deref().unwrap_or("").contains("101/100"));
}

#[test]
fn a_wrong_priority_or_health_check_is_a_failed_keepalived_row() {
    let descriptor = descriptor();
    let text = ingress::keepalived_conf(&descriptor, "edge-1", 0).expect("config");
    assert!(keepalived_text_issues(&text, &descriptor, "edge-1", 0).is_empty());
    let broken = text
        .replace("priority 101", "priority 100")
        .replace("nc -z 127.0.0.1 443", "true");
    let issues = keepalived_text_issues(&broken, &descriptor, "edge-1", 0);
    assert!(
        issues.iter().any(|issue| issue.contains("priority 101")),
        "{issues:?}"
    );
    assert!(
        issues.iter().any(|issue| issue.contains("nc -z")),
        "{issues:?}"
    );
}

#[test]
fn certificate_files_must_be_readable_and_the_key_mode_600() {
    let (certificate, key) = cert_files();
    let descriptor = descriptor_with_files(&certificate, &key);
    let rows = local_rows_with(&descriptor, &healthy_certificate());
    assert!(
        rows.iter().all(|row| !row.is_failure()),
        "{}",
        render(&rows)
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o644)).expect("key mode");
        let rows = local_rows_with(&descriptor, &healthy_certificate());
        assert!(
            rows.iter()
                .any(|row| row.is_failure()
                    && row.error.as_deref().unwrap_or("").contains("mode 600")),
            "{}",
            render(&rows)
        );
        let _ = std::fs::remove_file(certificate);
        let _ = std::fs::remove_file(key);
    }
}

#[test]
fn a_wrong_san_or_an_expiring_certificate_fails() {
    let (certificate, key) = cert_files();
    let descriptor = descriptor_with_files(&certificate, &key);
    let wrong = FakeCertificate {
        facts: Ok(CertificateFacts {
            dns_names: vec!["example.invalid".to_owned()],
            valid_for_days: true,
        }),
    };
    let rows = local_rows_with(&descriptor, &wrong);
    assert!(
        rows.iter().any(|row| row.state == "invalid"),
        "{}",
        render(&rows)
    );
    let expiring = FakeCertificate {
        facts: Ok(CertificateFacts {
            dns_names: vec!["quantrule.dev".to_owned()],
            valid_for_days: false,
        }),
    };
    let rows = local_rows_with(&descriptor, &expiring);
    assert!(
        rows.iter().any(|row| row
            .error
            .as_deref()
            .unwrap_or("")
            .contains("within 30 days")),
        "{}",
        render(&rows)
    );
    let _ = std::fs::remove_file(certificate);
    let _ = std::fs::remove_file(key);
}

#[test]
fn a_missing_openssl_is_a_limited_failure_not_a_pass() {
    let (certificate, key) = cert_files();
    let descriptor = descriptor_with_files(&certificate, &key);
    let missing = FakeCertificate {
        facts: Err("cannot run openssl x509: No such file or directory; the SAN and the 30-day validity were not checked".to_owned()),
    };
    let rows = local_rows_with(&descriptor, &missing);
    let row = rows
        .iter()
        .find(|row| row.subject == "certificate")
        .expect("certificate row");
    assert!(row.is_failure());
    assert_eq!(row.state, "limited");
    assert!(row.error.as_deref().unwrap_or("").contains("cannot run"));
    let _ = std::fs::remove_file(certificate);
    let _ = std::fs::remove_file(key);
}

struct FakeProbe {
    responses: BTreeMap<String, Result<ProbeResponse, String>>,
}

impl Probe for FakeProbe {
    fn get(&self, url: &str) -> Result<ProbeResponse, String> {
        self.responses
            .get(url)
            .cloned()
            .unwrap_or_else(|| Err(format!("{FIXTURE_MISSING} no response for {url}")))
    }
}

fn headers() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "strict-transport-security".to_owned(),
            "max-age=31536000; includeSubDomains".to_owned(),
        ),
        ("x-content-type-options".to_owned(), "nosniff".to_owned()),
        ("referrer-policy".to_owned(), "no-referrer".to_owned()),
    ])
}

fn healthy_fixture(descriptor: &Descriptor) -> FakeProbe {
    let base = format!("https://{}", descriptor.ingress.dns);
    FakeProbe {
        responses: BTreeMap::from([
            (
                format!("{base}/health"),
                Ok(ProbeResponse {
                    status: 200,
                    headers: headers(),
                    body: br#"{"state":"healthy"}"#.to_vec(),
                }),
            ),
            (
                format!("{base}/ready"),
                Ok(ProbeResponse {
                    status: 200,
                    headers: BTreeMap::new(),
                    body: br#"{"state":"healthy"}"#.to_vec(),
                }),
            ),
            (
                format!("{base}/version"),
                Ok(ProbeResponse {
                    status: 200,
                    headers: BTreeMap::new(),
                    body: format!(r#"{{"image_digest":"{}"}}"#, descriptor.image.digest)
                        .into_bytes(),
                }),
            ),
            (
                format!("http://{}/", descriptor.ingress.dns),
                Err("connection refused".to_owned()),
            ),
        ]),
    }
}

fn probe_report(descriptor: &Descriptor, probe: &FakeProbe) -> CheckReport {
    let mut report = CheckReport::new(
        descriptor.profile.as_str(),
        "HA profile (unverified)",
        descriptor.image.digest.clone(),
    );
    probe_rows(&mut report, descriptor, probe);
    report
}

#[test]
fn a_healthy_probe_passes_headers_and_port_80_closure() {
    let descriptor = descriptor();
    let report = probe_report(&descriptor, &healthy_fixture(&descriptor));
    assert!(!report.failed(), "{}", report.render());
    let text = report.render();
    assert!(text.contains("response headers"), "{text}");
    assert!(text.contains("no HTTP response"), "{text}");
}

#[test]
fn a_missing_hsts_or_a_server_header_fails_the_header_row() {
    let descriptor = descriptor();
    let mut fixture = healthy_fixture(&descriptor);
    let health = format!("https://{}/health", descriptor.ingress.dns);
    fixture.responses.insert(
        health.clone(),
        Ok(ProbeResponse {
            status: 200,
            headers: BTreeMap::from([
                ("x-content-type-options".to_owned(), "nosniff".to_owned()),
                ("referrer-policy".to_owned(), "no-referrer".to_owned()),
                ("server".to_owned(), "traefik".to_owned()),
            ]),
            body: br#"{"state":"healthy"}"#.to_vec(),
        }),
    );
    let report = probe_report(&descriptor, &fixture);
    assert!(report.failed());
    let text = report.render();
    assert!(text.contains("HSTS"), "{text}");
    assert!(text.contains("Server header"), "{text}");
}

#[test]
fn an_answer_on_port_80_fails_and_a_missing_fixture_is_not_a_pass() {
    let descriptor = descriptor();
    let mut fixture = healthy_fixture(&descriptor);
    fixture.responses.insert(
        format!("http://{}/", descriptor.ingress.dns),
        Ok(ProbeResponse {
            status: 301,
            headers: BTreeMap::new(),
            body: Vec::new(),
        }),
    );
    let report = probe_report(&descriptor, &fixture);
    assert!(report.render().contains("port 80 answered HTTP 301"));

    let missing = FakeProbe {
        responses: BTreeMap::new(),
    };
    let report = probe_report(&descriptor, &missing);
    let text = report.render();
    assert!(text.contains("has no plain-HTTP entry"), "{text}");
}

#[test]
fn a_failed_https_probe_does_not_claim_port_80_closure() {
    let descriptor = descriptor();
    let mut fixture = healthy_fixture(&descriptor);
    fixture.responses.insert(
        format!("https://{}/ready", descriptor.ingress.dns),
        Ok(ProbeResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: br#"{"state":"down"}"#.to_vec(),
        }),
    );
    let report = probe_report(&descriptor, &fixture);
    assert!(report.render().contains("not-attempted"));
}

#[derive(Default)]
struct FakeDocker {
    service: Option<Result<String, String>>,
}

impl DockerQuery for FakeDocker {
    fn swarm_info(&self) -> Result<String, String> {
        Err("unused".to_owned())
    }
    fn node_ids(&self) -> Result<Vec<String>, String> {
        Err("unused".to_owned())
    }
    fn node_inspect(&self, _id: &str) -> Result<String, String> {
        Err("unused".to_owned())
    }
    fn network_inspect(&self, _name: &str) -> Result<String, String> {
        Err("unused".to_owned())
    }
    fn service_inspect(&self, _name: &str) -> Result<String, String> {
        self.service
            .clone()
            .unwrap_or_else(|| Err("missing fixture".to_owned()))
    }
    fn service_tasks(&self, _name: &str) -> Result<String, String> {
        Err("unused".to_owned())
    }
    fn runtime_probe(&self, _network: &str, _image: &str, _url: &str) -> Result<String, String> {
        Err("unused".to_owned())
    }
}

fn ingress_service(configs: serde_json::Value, secrets: serde_json::Value) -> String {
    serde_json::json!({
        "Spec": {
            "TaskTemplate": {
                "ContainerSpec": {
                    "Image": "traefik@sha256:x",
                    "Configs": configs,
                    "Secrets": secrets
                }
            }
        }
    })
    .to_string()
}

fn mounted(name: &str, target: &str, kind: &str) -> serde_json::Value {
    if kind == "config" {
        serde_json::json!({
            "ConfigName": name,
            "File": {"Name": target, "Mode": 292}
        })
    } else {
        serde_json::json!({
            "SecretName": name,
            "File": {"Name": target, "Mode": 292}
        })
    }
}

#[test]
fn the_four_ingress_objects_must_be_mounted_at_the_documented_targets() {
    let descriptor = descriptor();
    let configs = serde_json::json!([
        mounted(TRAEFIK_CONFIG, "/etc/traefik/traefik.yml", "config"),
        mounted(
            TRAEFIK_DYNAMIC_CONFIG,
            "/etc/traefik/dynamic/dynamic.yml",
            "config"
        ),
        mounted(
            INGRESS_CERT_CONFIG,
            "/etc/traefik/certs/ingress.crt",
            "config"
        )
    ]);
    let secrets = serde_json::json!([mounted(
        INGRESS_KEY_SECRET,
        "/etc/traefik/certs/ingress.key",
        "secret"
    )]);
    let fake = FakeDocker {
        service: Some(Ok(ingress_service(configs, secrets))),
    };
    let rows = remote_rows(&descriptor, &fake);
    assert!(
        rows.iter().all(|row| !row.is_failure()),
        "{}",
        render(&rows)
    );
    assert!(rows.iter().any(|row| row.state == "limited"));
}

#[test]
fn a_versioned_dynamic_config_name_is_accepted_at_the_target() {
    let descriptor = descriptor();
    let configs = serde_json::json!([
        mounted(TRAEFIK_CONFIG, "/etc/traefik/traefik.yml", "config"),
        mounted(
            "quantrule_traefik_dynamic_v4",
            "/etc/traefik/dynamic/dynamic.yml",
            "config"
        ),
        mounted(
            INGRESS_CERT_CONFIG,
            "/etc/traefik/certs/ingress.crt",
            "config"
        )
    ]);
    let secrets = serde_json::json!([mounted(
        INGRESS_KEY_SECRET,
        "/etc/traefik/certs/ingress.key",
        "secret"
    )]);
    let fake = FakeDocker {
        service: Some(Ok(ingress_service(configs, secrets))),
    };
    let rows = remote_rows(&descriptor, &fake);
    let row = rows
        .iter()
        .find(|row| row.subject == "service mounts")
        .expect("mount row");
    assert!(!row.is_failure(), "{row:?}");
    assert_eq!(row.state, "mounted");
}

#[test]
fn a_missing_or_wrong_mount_is_a_failed_row() {
    let descriptor = descriptor();
    let configs = serde_json::json!([mounted(
        TRAEFIK_CONFIG,
        "/etc/traefik/traefik.yml",
        "config"
    )]);
    let fake = FakeDocker {
        service: Some(Ok(ingress_service(configs, serde_json::json!([])))),
    };
    let rows = remote_rows(&descriptor, &fake);
    let row = rows
        .iter()
        .find(|row| row.subject == "service mounts")
        .expect("mount row");
    assert!(row.is_failure());
    let error = row.error.as_deref().unwrap_or("");
    assert!(error.contains("quantrule_traefik_dynamic"), "{error}");
    assert!(error.contains("quantrule_ingress_key"), "{error}");
}

#[test]
fn a_missing_ingress_service_is_a_failed_row_and_not_a_panic() {
    let descriptor = descriptor();
    let rows = remote_rows(&descriptor, &FakeDocker::default());
    assert!(rows.iter().any(|row| row.is_failure()));
    assert!(rows.iter().any(|row| row.state == "limited"));
}

#[test]
fn the_dns_parser_keeps_every_san_name() {
    let text = "X509v3 Subject Alternative Name:\n    DNS:quantrule.dev, DNS:www.quantrule.dev\n";
    assert_eq!(
        parse_dns_names(text),
        vec!["quantrule.dev", "www.quantrule.dev"]
    );
}

fn render(rows: &[CheckRow]) -> String {
    rows.iter()
        .map(|row| {
            format!(
                "{} {} {} {}",
                row.status(),
                row.subject,
                row.state,
                row.error.as_deref().or(row.detail.as_deref()).unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
