use super::*;
use crate::descriptor::v3::Watchdog;
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
    value["images"]
        .as_object_mut()
        .expect("images")
        .remove("minio_client");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.images.missing"),
        "{error}"
    );
    assert!(error.to_string().contains("images.minio_client"), "{error}");

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
fn a_bare_app_image_reference_is_refused() {
    let mut value = sample();
    value["image"]["reference"] = serde_json::json!("quantrule-server");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.image.reference"),
        "{error}"
    );
    assert!(error.to_string().contains("registry/repository"), "{error}");
}

#[test]
fn release_mode_refuses_the_labelled_fixture_digest() {
    let example = valid();
    validate(&example).expect("the fixture remains valid by default");
    let error = validate_release(&example).expect_err("release mode refuses the fixture");
    assert!(
        error.to_string().contains("deploy.image.fixture"),
        "{error}"
    );

    let mut value = sample();
    let real = format!("sha256:{}", "9".repeat(64));
    value["image"]["digest"] = serde_json::json!(real.clone());
    value["images"]["minio"]["digest"] = serde_json::json!(real.clone());
    value["images"]["minio_client"]["digest"] = serde_json::json!(real);
    validate_release(&descriptor(value)).expect("a fully pinned descriptor is accepted");
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
fn rabbitmq_nodes_must_be_odd_and_at_least_three() {
    let mut value = sample();
    value["stateful"]["rabbitmq"]["nodes"] =
        serde_json::json!(["infra-1", "infra-2", "infra-3", "infra-4"]);
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.stateful.rabbitmq.nodes"),
        "{error}"
    );
    assert!(error.to_string().contains("odd"), "{error}");
}

#[test]
fn rabbitmq_alarm_defaults_and_ranges_are_enforced() {
    let mut value = sample();
    value["stateful"]["rabbitmq"]
        .as_object_mut()
        .expect("rabbitmq")
        .remove("disk_free_limit_mib");
    value["stateful"]["rabbitmq"]
        .as_object_mut()
        .expect("rabbitmq")
        .remove("memory_high_watermark_percent");
    let descriptor = descriptor(value);
    assert_eq!(descriptor.stateful.rabbitmq.disk_free_limit_mib, 5120);
    assert_eq!(
        descriptor.stateful.rabbitmq.memory_high_watermark_percent,
        60
    );
    validate(&descriptor).expect("the defaults validate");

    for (field, bad, code) in [
        (
            "disk_free_limit_mib",
            serde_json::json!(0),
            "deploy.stateful.rabbitmq.disk_free_limit_mib",
        ),
        (
            "disk_free_limit_mib",
            serde_json::json!(10_485_761),
            "deploy.stateful.rabbitmq.disk_free_limit_mib",
        ),
        (
            "memory_high_watermark_percent",
            serde_json::json!(9),
            "deploy.stateful.rabbitmq.memory_high_watermark_percent",
        ),
        (
            "memory_high_watermark_percent",
            serde_json::json!(91),
            "deploy.stateful.rabbitmq.memory_high_watermark_percent",
        ),
    ] {
        let mut value = sample();
        value["stateful"]["rabbitmq"][field] = bad;
        let error = reject(value);
        assert!(error.to_string().contains(code), "{field}: {error}");
    }
}

#[test]
fn the_queue_endpoint_must_use_the_shared_rabbitmq_alias() {
    let mut value = sample();
    value["endpoints"]["queue"]["host"] = serde_json::json!("rabbitmq-infra-1");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.endpoint.queue.host"),
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
        ("minio", 4),
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
            value["stateful"][unit]["drives"] = serde_json::json!([1, 1, 1]);
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
    value["profile"] = serde_json::json!("lab-single-host");
    value["stateful"]["minio"]["nodes"] = serde_json::json!(["infra-1", "infra-2", "infra-3"]);
    value["stateful"]["minio"]["drives"] = serde_json::json!([1, 1, 1]);
    let error = reject(value);
    assert!(
        error.to_string().contains("at least four drives"),
        "{error}"
    );

    let mut value = sample();
    value["stateful"]["minio"]["drives"] = serde_json::json!([1, 1, 1]);
    let error = reject(value);
    assert!(
        error.to_string().contains("one drive count per node"),
        "{error}"
    );

    let mut value = sample();
    value["stateful"]["minio"]["drives"] = serde_json::json!([1, 1, 1, 2]);
    let error = reject(value);
    assert!(error.to_string().contains("one drive on each"), "{error}");
}

#[test]
fn the_lab_may_run_minio_on_three_vms_with_two_drives_each() {
    let mut value = sample();
    value["profile"] = serde_json::json!("lab-single-host");
    value["stateful"]["minio"]["nodes"] = serde_json::json!(["infra-1", "infra-2", "infra-3"]);
    value["stateful"]["minio"]["drives"] = serde_json::json!([2, 2, 2]);
    value["stateful"]["minio"]["parity"] = serde_json::json!("EC:3");
    let lab = descriptor(value);
    validate(&lab).expect("the lab uses three MinIO VMs with two drives each");
    assert_eq!(minio_parity(&lab).expect("parity"), 3);
}

#[test]
fn minio_parity_defaults_to_auto_and_keeps_the_write_quorum() {
    let sample_descriptor = valid(); // explicit EC:2
    assert_eq!(minio_parity(&sample_descriptor).expect("parity"), 2);

    let mut value = sample();
    value["stateful"]["minio"]
        .as_object_mut()
        .expect("minio")
        .remove("parity");
    let auto = descriptor(value);
    assert_eq!(auto.stateful.minio.parity, "auto");
    assert_eq!(minio_parity(&auto).expect("auto parity"), 2);

    let mut value = sample();
    value["profile"] = serde_json::json!("lab-single-host");
    value["stateful"]["minio"]["nodes"] = serde_json::json!(["infra-1", "infra-2", "infra-3"]);
    value["stateful"]["minio"]["drives"] = serde_json::json!([2, 1, 1]);
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.stateful.minio.parity"),
        "{error}"
    );
    assert!(error.to_string().contains("write quorum"), "{error}");

    let mut value = sample();
    value["profile"] = serde_json::json!("lab-single-host");
    value["stateful"]["minio"]["nodes"] = serde_json::json!(["infra-1", "infra-2", "infra-3"]);
    value["stateful"]["minio"]["drives"] = serde_json::json!([2, 2, 2]);
    value["stateful"]["minio"]["parity"] = serde_json::json!("EC:3");
    let lab = descriptor(value);
    assert_eq!(minio_parity(&lab).expect("lab parity"), 3);
}

#[test]
fn minio_parity_is_bounded_by_the_drive_count_and_the_data_blocks() {
    for (parity, code) in [
        ("EC:0", "at least one parity drive"),
        ("EC:1", "write quorum"),
        ("EC:4", "data blocks"),
        ("nope", "auto or EC:<n>"),
        ("EC:x", "auto or EC:<n>"),
    ] {
        let mut value = sample();
        value["stateful"]["minio"]["parity"] = serde_json::json!(parity);
        let error = reject(value);
        assert!(
            error.to_string().contains("deploy.stateful.minio.parity"),
            "{parity}: {error}"
        );
        assert!(error.to_string().contains(code), "{parity}: {error}");
    }
}

#[test]
fn object_replication_must_be_false_until_ha013_lands() {
    let mut value = sample();
    value["backup"]["object_replication"] = serde_json::json!(true);
    let error = reject(value);
    assert!(
        error
            .to_string()
            .contains("deploy.backup.object_replication"),
        "{error}"
    );
    assert!(error.to_string().contains("HA-013"), "{error}");
    assert!(!valid().backup.object_replication);
}

#[test]
fn minio_members_keep_distinct_failure_domains() {
    let mut value = sample();
    value["hosts"]["infra-4"]["failure_domain"] = serde_json::json!("az1");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.stateful.anti_affinity"),
        "{error}"
    );
}

#[test]
fn the_swarm_control_plane_needs_three_odd_distinct_declared_managers() {
    let mut value = sample();
    value["swarm"]["managers"] = serde_json::json!([]);
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.swarm.managers"),
        "{error}"
    );

    let mut value = sample();
    value["swarm"]["managers"] = serde_json::json!(["edge-1", "core-1"]);
    let error = reject(value);
    assert!(error.to_string().contains("odd number"), "{error}");

    let mut value = sample();
    value["swarm"]["managers"] = serde_json::json!(["edge-1", "core-1", "edge-1"]);
    let error = reject(value);
    assert!(error.to_string().contains("listed twice"), "{error}");

    let mut value = sample();
    value["swarm"]["managers"] = serde_json::json!(["edge-1", "core-1", "missing"]);
    let error = reject(value);
    assert!(
        error.to_string().contains("not declared in hosts"),
        "{error}"
    );

    let mut value = sample();
    value["profile"] = serde_json::json!("lab-single-host");
    validate(&descriptor(value)).expect("the lab uses its three VMs as managers");
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
    value["hosts"]["infra-4"]["failure_domain"] = serde_json::json!("az1");
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
fn the_ingress_vip_is_a_required_valid_distinct_ipv4() {
    let mut value = sample();
    value["ingress"]
        .as_object_mut()
        .expect("ingress")
        .remove("vip");
    let error = reject(value);
    assert!(error.to_string().contains("deploy.ingress.vip"), "{error}");

    for bad in ["", "not-an-ip", "203.0.113.999", "2001:db8::10"] {
        let mut value = sample();
        value["ingress"]["vip"] = serde_json::json!(bad);
        let error = reject(value);
        assert!(
            error.to_string().contains("deploy.ingress.vip"),
            "{bad}: {error}"
        );
    }
    for bad in ["0.0.0.0", "127.0.0.1", "224.0.0.1", "255.255.255.255"] {
        let mut value = sample();
        value["ingress"]["vip"] = serde_json::json!(bad);
        let error = reject(value);
        assert!(
            error.to_string().contains("deploy.ingress.vip"),
            "{bad}: {error}"
        );
    }
    let mut value = sample();
    value["ingress"]["vip"] = serde_json::json!("203.0.113.20");
    validate(&descriptor(value)).expect("a distinct public IPv4 VIP");
}

#[test]
fn the_ingress_vip_cannot_equal_an_ingress_public_address() {
    for host in ["edge-1", "edge-2"] {
        let public = sample()["hosts"][host]["public_address"]
            .as_str()
            .expect("public address")
            .to_owned();
        let mut value = sample();
        value["ingress"]["vip"] = serde_json::json!(public);
        let error = reject(value);
        assert!(
            error.to_string().contains("deploy.ingress.vip"),
            "{host}: {error}"
        );
        assert!(error.to_string().contains(host), "{host}: {error}");
    }
}

#[test]
fn the_ingress_interface_defaults_to_eth0_and_is_narrow() {
    let mut value = sample();
    value["ingress"]
        .as_object_mut()
        .expect("ingress")
        .remove("interface");
    let parsed = descriptor(value);
    assert_eq!(parsed.ingress.interface, "eth0");
    validate(&parsed).expect("the default interface");

    for bad in ["", "eth 0", "eth/0", "eth0:1", "..", "1234567890123456"] {
        let mut value = sample();
        value["ingress"]["interface"] = serde_json::json!(bad);
        let error = reject(value);
        assert!(
            error.to_string().contains("deploy.ingress.interface"),
            "{bad:?}: {error}"
        );
    }
    for good in ["eth0", "enp3s0", "eth0.100", "br-abc_1"] {
        let mut value = sample();
        value["ingress"]["interface"] = serde_json::json!(good);
        validate(&descriptor(value)).expect("a simple interface name");
    }
}

#[test]
fn acme_is_reserved_and_refused_until_the_future_path() {
    for files in [
        serde_json::json!({}),
        serde_json::json!({"certificate": "/etc/quantrule/tls/fullchain.pem"}),
    ] {
        let mut value = sample();
        value["ingress"]["tls"] = serde_json::json!("acme");
        value["ingress"]["files"] = files;
        let error = reject(value);
        assert!(error.to_string().contains("deploy.ingress.tls"), "{error}");
        assert!(error.to_string().contains("acme"), "{error}");
        assert!(error.to_string().contains("cert"), "{error}");
    }
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
fn postgres_synchronous_standby_is_exactly_one() {
    for value in [0, 2] {
        let mut value_json = sample();
        value_json["stateful"]["postgres"]["synchronous_standby"] = serde_json::json!(value);
        let error = reject(value_json);
        assert!(
            error
                .to_string()
                .contains("deploy.stateful.postgres.synchronous_standby"),
            "{value}: {error}"
        );
    }
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
fn the_watchdog_defaults_to_off_and_accepts_the_three_modes() {
    let mut value = sample();
    value["stateful"]["postgres"]
        .as_object_mut()
        .expect("postgres")
        .remove("watchdog");
    let parsed = descriptor(value);
    assert_eq!(parsed.stateful.postgres.watchdog, Watchdog::Off);
    validate(&parsed).expect("the default watchdog");

    for (spelling, expected) in [
        ("off", Watchdog::Off),
        ("automatic", Watchdog::Automatic),
        ("required", Watchdog::Required),
    ] {
        let mut value = sample();
        value["stateful"]["postgres"]["watchdog"] = serde_json::json!(spelling);
        assert_eq!(descriptor(value).stateful.postgres.watchdog, expected);
    }

    let mut value = sample();
    value["stateful"]["postgres"]["watchdog"] = serde_json::json!("sometimes");
    let error = serde_json::from_value::<Descriptor>(value).expect_err("unknown watchdog");
    assert!(error.to_string().contains("unknown variant"), "{error}");
}

#[test]
fn the_lab_descriptor_validates() {
    let lab: Descriptor =
        serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.lab.json"))
            .expect("the lab descriptor is JSON");
    validate(&lab).expect("the lab descriptor validates");
    assert_eq!(lab.profile, Profile::LabSingleHost);
    assert_eq!(lab.stateful.postgres.watchdog, Watchdog::Off);
    assert_eq!(lab.hosts.len(), 3);
}

#[test]
fn patroni_is_a_required_infrastructure_pin() {
    let mut value = sample();
    value["images"]
        .as_object_mut()
        .expect("images")
        .remove("patroni");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.images.missing"),
        "{error}"
    );
    assert!(error.to_string().contains("images.patroni"), "{error}");
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
fn observability_image_pins_are_required_and_closed() {
    assert_eq!(valid().images.len(), 16);
    assert_eq!(REQUIRED_IMAGES.len(), 16);
    for name in ["node_exporter", "promtail", "blackbox_exporter"] {
        let mut value = sample();
        value["images"]
            .as_object_mut()
            .expect("images")
            .remove(name);
        let error = reject(value);
        assert!(
            error.to_string().contains("deploy.images.missing"),
            "{name}: {error}"
        );
        assert!(error.to_string().contains(name), "{name}: {error}");
    }
    let mut value = sample();
    value["images"]
        .as_object_mut()
        .expect("images")
        .remove("traefik");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.images.missing"),
        "{error}"
    );
}

#[test]
fn observability_keeps_prometheus_and_alertmanager_on_distinct_hosts() {
    let mut value = sample();
    value["observability"]["alertmanager"]["host"] = serde_json::json!("infra-1");
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.observability.placement"),
        "{error}"
    );
    assert!(
        error
            .to_string()
            .contains("observability.alertmanager.host"),
        "{error}"
    );
}

#[test]
fn observability_refuses_all_four_units_on_one_host() {
    let mut value = sample();
    for unit in ["prometheus", "grafana", "loki", "alertmanager"] {
        value["observability"][unit]["host"] = serde_json::json!("infra-1");
    }
    let error = reject(value);
    assert!(
        error.to_string().contains("deploy.observability.placement"),
        "{error}"
    );
    assert!(error.to_string().contains("one host"), "{error}");
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
