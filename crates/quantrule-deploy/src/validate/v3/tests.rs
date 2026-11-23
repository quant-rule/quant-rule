use super::*;
use serde_json::Value;

fn sample() -> Value {
    serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.example.json"))
        .expect("the v3 example is JSON")
}

fn descriptor(value: Value) -> Descriptor {
    serde_json::from_value(value).expect("the mutated example stays structurally valid")
}

fn valid() -> Descriptor {
    descriptor(sample())
}

fn reject(value: Value) -> Error {
    validate(&descriptor(value)).expect_err("the descriptor must be refused")
}

#[test]
fn the_example_validates() {
    validate(&valid()).expect("the example validates");
}

#[test]
fn the_version_is_stable() {
    let mut value = sample();
    value["v"] = serde_json::json!("quantrule.deploy/v4");
    let error = reject(value);
    assert!(error.to_string().contains("deploy.version"), "{error}");
}

#[test]
fn the_app_and_every_infra_unit_need_a_digest() {
    let mut value = sample();
    value["images"]["postgres"]["digest"] = serde_json::json!(
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    let error = reject(value);
    assert!(error.to_string().contains("placeholder"), "{error}");
    assert!(
        error.to_string().contains("images.postgres.digest"),
        "{error}"
    );
}

#[test]
fn a_missing_or_unknown_infra_pin_is_refused() {
    let mut value = sample();
    value["images"]
        .as_object_mut()
        .expect("images")
        .remove("etcd");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.images.missing"),
        "{error}"
    );
    assert!(error.to_string().contains("images.etcd"), "{error}");

    let mut value = sample();
    value["images"]["surprise"] = serde_json::json!({
        "reference": "example/surprise",
        "digest": format!("sha256:{}", "4".repeat(64)),
    });
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.images.unknown"),
        "{error}"
    );
}

#[test]
fn a_tag_only_app_image_is_refused() {
    let mut value = sample();
    value["image"]["digest"] = serde_json::json!("registry/server:latest");
    let error = reject(value);
    assert!(error.to_string().contains("sha256:"), "{error}");
}

#[test]
fn exactly_seven_services_are_required() {
    let mut value = sample();
    value["services"]
        .as_object_mut()
        .expect("services")
        .remove("strategy");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.service.missing"),
        "{error}"
    );
    assert!(error.to_string().contains("strategy"), "{error}");

    let mut value = sample();
    value["services"]["surprise"] = serde_json::json!({
        "replicas": 1,
        "hosts": ["compute-1"],
        "port": 19000,
    });
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.service.unknown"),
        "{error}"
    );
}

#[test]
fn exactly_four_databases_with_fixed_roles_are_required() {
    let mut value = sample();
    value["databases"]
        .as_object_mut()
        .expect("databases")
        .remove("catalog");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.database.missing"),
        "{error}"
    );

    let mut value = sample();
    value["databases"]["gateway"]["app_role"] = serde_json::json!("qr_catalog_app");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.database.app_role"),
        "{error}"
    );
}

#[test]
fn a_service_port_is_never_443() {
    let mut value = sample();
    value["services"]["gateway"]["port"] = serde_json::json!(443);
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.service.public_port"),
        "{error}"
    );
}

#[test]
fn a_service_host_must_be_declared() {
    let mut value = sample();
    value["services"]["data"]["hosts"] = serde_json::json!(["missing"]);
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.host.undeclared"),
        "{error}"
    );
}

#[test]
fn an_undeclared_stateful_node_is_refused() {
    let mut value = sample();
    value["stateful"]["etcd"]["nodes"] = serde_json::json!(["infra-1", "infra-2", "missing"]);
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.host.undeclared"),
        "{error}"
    );
}

#[test]
fn a_stateful_unit_below_quorum_is_refused() {
    for (unit, minimum) in [
        ("postgres", 3),
        ("etcd", 3),
        ("haproxy", 2),
        ("rabbitmq", 3),
        ("minio", 3),
    ] {
        let mut value = sample();
        let nodes = value["stateful"][unit]["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .take(minimum - 1)
            .cloned()
            .collect::<Vec<_>>();
        value["stateful"][unit]["nodes"] = Value::Array(nodes);
        if unit == "minio" {
            value["stateful"][unit]["drives"] = serde_json::json!([1, 1]);
        }
        let error = reject(value);
        assert!(
            error.to_string().contains("deploy.stateful.nodes"),
            "{unit}: {error}"
        );
    }
}

#[test]
fn minio_needs_four_drives_and_one_count_per_node() {
    let mut value = sample();
    value["stateful"]["minio"]["drives"] = serde_json::json!([1, 1, 1]);
    let error = reject(value);
    assert!(
        error.to_string().contains("at least four drives"),
        "{error}"
    );

    let mut value = sample();
    value["stateful"]["minio"]["drives"] = serde_json::json!([2, 1]);
    let error = reject(value);
    assert!(
        error.to_string().contains("one drive count per node"),
        "{error}"
    );
}

#[test]
fn no_reschedule_field_is_accepted() {
    let mut value = sample();
    value["stateful"]["postgres"]["reschedule"] = serde_json::json!(true);
    let error = serde_json::from_value::<Descriptor>(value).expect_err("reschedule");
    assert!(error.to_string().contains("unknown field"), "{error}");
}

#[test]
fn the_profiles_enforce_their_failure_domain_counts() {
    let mut value = sample();
    value["hosts"]["infra-3"]["failure_domain"] = serde_json::json!("az1");
    value["hosts"]["compute-3"]["failure_domain"] = serde_json::json!("az2");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.profile.failure_domains"),
        "{error}"
    );
}

#[test]
fn single_az_requires_host_and_domain_anti_affinity() {
    let mut value = sample();
    value["profile"] = serde_json::json!("single-az");
    validate(&descriptor(value)).expect("single-az keeps the strict anti-affinity rules");

    let mut value = sample();
    value["profile"] = serde_json::json!("single-az");
    for host in value["hosts"].as_object_mut().expect("hosts").values_mut() {
        host["failure_domain"] = serde_json::json!("az1");
    }
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.service.anti_affinity"),
        "{error}"
    );

    let mut value = sample();
    value["profile"] = serde_json::json!("single-az");
    value["services"]["gateway"]["hosts"] = serde_json::json!(["edge-1"]);
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.service.anti_affinity"),
        "{error}"
    );
}

#[test]
fn lab_single_host_allows_co_location() {
    let mut value = sample();
    value["profile"] = serde_json::json!("lab-single-host");
    for host in value["hosts"].as_object_mut().expect("hosts").values_mut() {
        host["failure_domain"] = serde_json::json!("lab");
    }
    value["services"]["gateway"]["hosts"] = serde_json::json!(["edge-1"]);
    value["services"]["model"]["hosts"] = serde_json::json!(["compute-1"]);
    value["services"]["data"]["hosts"] = serde_json::json!(["core-1"]);
    value["services"]["evidence"]["hosts"] = serde_json::json!(["core-1"]);
    value["services"]["simulation"]["hosts"] = serde_json::json!(["core-1"]);
    validate(&descriptor(value)).expect("the lab profile allows service co-location");
}

#[test]
fn a_service_cannot_have_more_replicas_than_hosts() {
    let mut value = sample();
    value["services"]["gateway"]["hosts"] = serde_json::json!(["edge-1"]);
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.service.anti_affinity"),
        "{error}"
    );
}

#[test]
fn swarm_ha_spreads_replicas_across_domains() {
    let mut value = sample();
    value["hosts"]["compute-3"]["failure_domain"] = serde_json::json!("az1");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.service.anti_affinity"),
        "{error}"
    );
}

#[test]
fn the_ingress_is_443_only_and_needs_public_addresses() {
    for port in [80, 8080, 8443] {
        let mut value = sample();
        value["ingress"]["port"] = serde_json::json!(port);
        let error = reject(value);
        assert!(
            error.to_string().contains("deploy.ingress.port"),
            "{port}: {error}"
        );
    }

    let mut value = sample();
    value["hosts"]["edge-2"]
        .as_object_mut()
        .expect("edge-2")
        .remove("public_address");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.ingress.public_address"),
        "{error}"
    );
}

#[test]
fn the_ingress_certificate_mode_is_closed() {
    let mut value = sample();
    value["ingress"]["tls"] = serde_json::json!("cert");
    value["ingress"]["files"] = serde_json::json!({
        "certificate": "/etc/quantrule/tls/fullchain.pem"
    });
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.ingress.files"),
        "{error}"
    );

    let mut value = sample();
    value["ingress"]["files"] = serde_json::json!({
        "certificate": "/etc/quantrule/tls/fullchain.pem"
    });
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.ingress.files"),
        "{error}"
    );

    let mut value = sample();
    value["ingress"]["tls"] = serde_json::json!("cert");
    value["ingress"]["files"] = serde_json::json!({
        "certificate": "tls/fullchain.pem",
        "key": "/etc/quantrule/tls/privkey.pem"
    });
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.file.absolute"),
        "{error}"
    );
}

#[test]
fn stateful_clients_require_tls_and_absolute_cas() {
    for (field, bad) in [
        ("endpoints.database.sslmode", serde_json::json!("require")),
        ("endpoints.database.port", serde_json::json!(5433)),
        ("endpoints.queue.port", serde_json::json!(5672)),
        ("endpoints.queue.scheme", serde_json::json!("amqp")),
        ("endpoints.object_store.port", serde_json::json!(9001)),
        ("endpoints.object_store.scheme", serde_json::json!("http")),
        ("endpoints.database.tls_ca", serde_json::json!("pki/ca.crt")),
    ] {
        let mut value = sample();
        let (section, key) = field
            .split_once('.')
            .and_then(|(_, rest)| rest.split_once('.'))
            .expect("field");
        value["endpoints"][section][key] = bad;
        let error = reject(value);
        assert!(
            error.to_string().contains(field) || error.to_string().contains("deploy.endpoint"),
            "{field}: {error}"
        );
    }
    let mut value = sample();
    value["endpoints"]["database"]["host"] = serde_json::json!("user@postgres");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.endpoint.host"),
        "{error}"
    );
}

#[test]
fn the_network_must_be_encrypted_and_the_ingress_name_is_fixed() {
    let mut value = sample();
    value["network"]["encrypted"] = serde_json::json!(false);
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.network.encrypted"),
        "{error}"
    );

    let mut value = sample();
    value["ingress"]["dns"] = serde_json::json!("example.invalid");
    let error = reject(value);
    assert!(error.to_string().contains("deploy.ingress.dns"), "{error}");
}

#[test]
fn postgres_synchronous_standby_is_bounded_by_the_nodes() {
    let mut value = sample();
    value["stateful"]["postgres"]["synchronous_standby"] = serde_json::json!(3);
    let error = reject(value);
    assert!(
        error
            .to_string()
            .contains("deploy.stateful.postgres.synchronous_standby"),
        "{error}"
    );
}

#[test]
fn swarm_ha_replicates_objects() {
    let mut value = sample();
    value["backup"]["object_replication"] = serde_json::json!(false);
    let error = reject(value);
    assert!(
        error
            .to_string()
            .contains("deploy.backup.object_replication"),
        "{error}"
    );
}

#[test]
fn a_public_address_on_a_non_ingress_host_is_refused() {
    let mut value = sample();
    value["hosts"]["core-1"]["public_address"] = serde_json::json!("203.0.113.20");
    let error = reject(value);
    assert!(
        error.to_string().contains("only an ingress host"),
        "{error}"
    );
}

#[test]
fn a_public_overlay_address_is_refused() {
    let mut value = sample();
    value["hosts"]["core-1"]["address"] = serde_json::json!("8.8.8.8");
    let error = reject(value);
    assert!(error.to_string().contains("private overlay"), "{error}");
}

#[test]
fn the_gateway_token_secret_must_be_declared() {
    let mut value = sample();
    value["auth"]["gateway_token_secret"] = serde_json::json!("missing_token");
    let error = reject(value);
    assert!(
        error
            .to_string()
            .contains("deploy.auth.gateway_token_secret"),
        "{error}"
    );
}

#[test]
fn backup_pitr_and_retention_are_required() {
    let mut value = sample();
    value["backup"]["pitr"] = serde_json::json!(false);
    let error = reject(value);
    assert!(error.to_string().contains("deploy.backup.pitr"), "{error}");

    let mut value = sample();
    value["backup"]["retention_days"] = serde_json::json!(0);
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.backup.retention_days"),
        "{error}"
    );
}

#[test]
fn observability_names_declared_hosts() {
    let mut value = sample();
    value["observability"]["loki"]["host"] = serde_json::json!("missing");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.host.undeclared"),
        "{error}"
    );
}

#[test]
fn identifiers_and_secret_names_are_narrow() {
    for value in ["edge-1", "az1", "lab"] {
        assert!(is_identifier(value), "{value}");
    }
    for value in [
        "",
        "Edge-1",
        "-edge",
        "edge-",
        "edge_1",
        "a".repeat(64).as_str(),
    ] {
        assert!(!is_identifier(value), "{value}");
    }
    assert!(is_secret_name("gateway_token"));
    assert!(!is_secret_name("Gateway-Token"));
}
