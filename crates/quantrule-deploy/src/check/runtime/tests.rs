use std::collections::BTreeMap;

use super::*;

fn descriptor() -> Descriptor {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../deploy/v3/deploy.example.json"
    )))
    .expect("v3 example")
}

#[derive(Default)]
struct FakeDocker {
    responses: BTreeMap<String, Result<String, String>>,
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
        Err("unused".to_owned())
    }

    fn service_tasks(&self, _name: &str) -> Result<String, String> {
        Err("unused".to_owned())
    }

    fn runtime_probe(&self, _network: &str, _image: &str, url: &str) -> Result<String, String> {
        self.responses
            .get(url)
            .cloned()
            .unwrap_or_else(|| Err(format!("no runtime fixture for {url}")))
    }
}

fn healthy(descriptor: &Descriptor) -> FakeDocker {
    let mut fake = FakeDocker::default();
    for (service, port, path) in runtime_table() {
        fake.responses.insert(
            format!("http://{service}:{port}{path}"),
            Ok(format!(
                r#"{{"image_digest":"{}"}}"#,
                descriptor.image.digest
            )),
        );
    }
    fake
}

fn text(rows: &[CheckRow]) -> String {
    rows.iter()
        .map(|row| {
            format!(
                "{} {} {} {} {}",
                row.status(),
                row.group,
                row.subject,
                row.state,
                row.error.as_deref().or(row.detail.as_deref()).unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn failures(rows: &[CheckRow]) -> Vec<&CheckRow> {
    rows.iter().filter(|row| row.is_failure()).collect()
}

#[test]
fn the_probe_reads_backtest_health_and_every_version_endpoint() {
    assert_eq!(runtime_table().len(), 7);
    assert!(runtime_table().contains(&("backtest", 9100, "/health")));
    assert!(
        runtime_table()
            .iter()
            .filter(|(_, port, path)| *port == 8080 && *path == "/version")
            .count()
            == 6
    );
}

#[test]
fn the_runtime_probe_passes_all_seven_services() {
    let descriptor = descriptor();
    let fake = healthy(&descriptor);
    let rows = inspect_runtime(&descriptor, &fake).expect("runtime rows");
    assert!(failures(&rows).is_empty(), "{}", text(&rows));
    assert_eq!(rows.len(), 7);
    for service in [
        "gateway",
        "data",
        "model",
        "strategy",
        "backtest",
        "evidence",
        "simulation",
    ] {
        assert!(
            rows.iter()
                .any(|row| row.subject == format!("runtime {service}")
                    && row.detail.as_deref() == Some(descriptor.image.digest.as_str())),
            "{service}: {}",
            text(&rows)
        );
    }
}

#[test]
fn a_runtime_digest_mismatch_fails_closed() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    fake.responses.insert(
        "http://backtest:9100/health".to_owned(),
        Ok(r#"{"image_digest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}"#
            .to_owned()),
    );
    let rows = inspect_runtime(&descriptor, &fake).expect("runtime rows");
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.subject == "runtime backtest"
                && row
                    .error
                    .as_deref()
                    .is_some_and(|error| error.contains("expected"))),
        "{}",
        text(&rows)
    );
}

#[test]
fn a_missing_digest_is_a_failed_row() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    fake.responses.insert(
        "http://gateway:8080/version".to_owned(),
        Ok(r#"{"product":"1.5.2"}"#.to_owned()),
    );
    let rows = inspect_runtime(&descriptor, &fake).expect("runtime rows");
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.subject == "runtime gateway" && row.state == "missing"),
        "{}",
        text(&rows)
    );
}

#[test]
fn an_unavailable_probe_is_a_failed_row_not_a_skip() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    fake.responses.insert(
        "http://gateway:8080/version".to_owned(),
        Err("docker run failed".to_owned()),
    );
    let rows = inspect_runtime(&descriptor, &fake).expect("runtime rows");
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.subject == "runtime gateway"
                && row.state == "unavailable"
                && row
                    .error
                    .as_deref()
                    .is_some_and(|error| error.contains("deploy.remote.runtime"))),
        "{}",
        text(&rows)
    );
}
