//! Unit tests for configuration, secrets and validation.

use super::*;

fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

#[derive(Default)]
struct MapReader {
    files: BTreeMap<String, Vec<u8>>,
}

impl MapReader {
    fn new(files: &[(&str, &str)]) -> Self {
        Self {
            files: files
                .iter()
                .map(|(path, contents)| ((*path).to_owned(), contents.as_bytes().to_vec()))
                .collect(),
        }
    }
}

impl SecretFileReader for MapReader {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.files
            .get(path.to_str().unwrap_or_default())
            .cloned()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }
}

fn config_with_files(
    variables: &[(&str, &str)],
    files: &[(&str, &str)],
) -> Result<Config, ConfigError> {
    let reader = MapReader::new(files);
    Config::from_values_with_reader(&values(variables), &reader)
}

const SENTINEL_TOKEN: &str = "sentinel-0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

#[test]
fn the_application_and_owner_urls_are_separate() {
    let config = config_with_files(
        &[
            ("QR_DATABASE_URL_FILE", "/run/secrets/database_url"),
            (
                "QR_DATABASE_OWNER_URL_FILE",
                "/run/secrets/database_owner_url",
            ),
        ],
        &[
            (
                "/run/secrets/database_url",
                "postgres://qr_catalog_app:app@postgres:5432/qr_catalog\n",
            ),
            (
                "/run/secrets/database_owner_url",
                "postgres://qr_catalog_owner:owner@postgres:5432/qr_catalog\r\n",
            ),
        ],
    )
    .expect("config");

    assert_eq!(
        config.database_url,
        "postgres://qr_catalog_app:app@postgres:5432/qr_catalog"
    );
    assert_eq!(
        config.require_database_url().expect("a file-backed URL"),
        "postgres://qr_catalog_app:app@postgres:5432/qr_catalog"
    );
    assert_eq!(
        config.database_owner_url.as_deref(),
        Some("postgres://qr_catalog_owner:owner@postgres:5432/qr_catalog")
    );
}

#[test]
fn a_migration_only_owner_url_is_validated_with_the_ca() {
    let config = config_with_files(
        &[
            (
                "QR_DATABASE_OWNER_URL_FILE",
                "/run/secrets/database_owner_url",
            ),
            ("QR_DATABASE_TLS_CA_PATH", "/run/pki/ca.crt"),
        ],
        &[(
            "/run/secrets/database_owner_url",
            "postgres://qr_evidence_owner:owner@postgres:5432/qr_evidence?sslmode=verify-full",
        )],
    )
    .expect("the owner-only migration URL must validate against the CA");
    assert!(config.database_owner_url.is_some());
    assert!(!config.database_url_from_secret);

    let error = config_with_files(
        &[
            (
                "QR_DATABASE_OWNER_URL_FILE",
                "/run/secrets/database_owner_url",
            ),
            ("QR_DATABASE_TLS_CA_PATH", "/run/pki/ca.crt"),
        ],
        &[(
            "/run/secrets/database_owner_url",
            "postgres://qr_evidence_owner:owner@postgres:5432/qr_evidence",
        )],
    )
    .expect_err("an owner URL without verify-full must be refused");
    assert!(
        error.to_string().contains("QR_DATABASE_OWNER_URL_FILE"),
        "{error}"
    );
}

#[test]
fn a_missing_owner_url_is_unset() {
    let config = Config::from_map(&BTreeMap::new()).expect("config");
    assert_eq!(config.database_owner_url, None);
}

#[test]
fn debug_redacts_every_secret() {
    let config = config_with_files(
        &[
            ("QR_DATABASE_URL_FILE", "/run/secrets/database_url"),
            (
                "QR_DATABASE_OWNER_URL_FILE",
                "/run/secrets/database_owner_url",
            ),
            (
                "QR_OBJECT_STORE_ACCESS_KEY_FILE",
                "/run/secrets/object_store_access_key",
            ),
            (
                "QR_OBJECT_STORE_SECRET_KEY_FILE",
                "/run/secrets/object_store_secret_key",
            ),
            ("QR_QUEUE_URL_FILE", "/run/secrets/queue_url"),
            ("QR_BEARER_TOKEN_FILE", "/run/secrets/bearer_token"),
        ],
        &[
            (
                "/run/secrets/database_url",
                "postgres://qr_gateway_app:app-secret@postgres:5432/qr_gateway",
            ),
            (
                "/run/secrets/database_owner_url",
                "postgres://qr_gateway_owner:owner-secret@postgres:5432/qr_gateway",
            ),
            ("/run/secrets/object_store_access_key", "access-secret"),
            ("/run/secrets/object_store_secret_key", "object-secret"),
            (
                "/run/secrets/queue_url",
                "amqp://quantrule:queue-secret@rabbitmq:5672/%2f",
            ),
            ("/run/secrets/bearer_token", SENTINEL_TOKEN),
        ],
    )
    .expect("config");
    let rendered = format!("{config:?}");
    for secret in [
        "app-secret",
        "owner-secret",
        "access-secret",
        "object-secret",
        "queue-secret",
        SENTINEL_TOKEN,
    ] {
        assert!(!rendered.contains(secret), "{secret} leaked: {rendered}");
    }
}

#[test]
fn a_file_secret_resolves_to_its_value() {
    let config = config_with_files(
        &[("QR_QUEUE_URL_FILE", "/run/secrets/queue_url")],
        &[(
            "/run/secrets/queue_url",
            "amqp://quantrule:sentinel-queue-secret@rabbitmq:5672/%2f\n",
        )],
    )
    .expect("config");
    assert_eq!(
        config.queue_url,
        "amqp://quantrule:sentinel-queue-secret@rabbitmq:5672/%2f"
    );
}

#[test]
fn a_plain_secret_is_refused_without_echoing_it() {
    for (plain, _file) in SECRET_VARIABLES {
        let error = Config::from_map(&values(&[(plain, "sentinel-plain-secret")]))
            .expect_err("a plain secret must be refused");
        let rendered = error.to_string();
        assert!(rendered.contains(plain), "{rendered}");
        assert!(!rendered.contains("sentinel-plain-secret"), "{rendered}");
    }
}

#[test]
fn a_plain_and_file_secret_together_are_refused() {
    let error = config_with_files(
        &[
            (
                "QR_DATABASE_URL",
                "postgres://app:sentinel@postgres:5432/db",
            ),
            ("QR_DATABASE_URL_FILE", "/run/secrets/database_url"),
        ],
        &[(
            "/run/secrets/database_url",
            "postgres://app:other@postgres:5432/db",
        )],
    )
    .expect_err("both sources must be refused");
    let rendered = error.to_string();
    assert!(rendered.contains("QR_DATABASE_URL"), "{rendered}");
    assert!(!rendered.contains("sentinel"), "{rendered}");
}

#[test]
fn the_secret_file_contract_is_enforced() {
    for (contents, reason) in [
        ("", SecretError::Empty),
        ("\n", SecretError::Empty),
        ("\r\n", SecretError::Empty),
        (" leading", SecretError::Whitespace),
        ("trailing ", SecretError::Whitespace),
        ("one\ntwo", SecretError::EmbeddedNewline),
        ("one\r\ntwo", SecretError::EmbeddedNewline),
        ("nul\0byte", SecretError::Nul),
    ] {
        assert_eq!(
            parse_secret_contents(contents.as_bytes()),
            Err(reason),
            "{contents:?}"
        );
    }
    assert_eq!(
        parse_secret_contents(b"one trailing newline\n"),
        Ok("one trailing newline".to_owned())
    );
    assert_eq!(
        parse_secret_contents(b"one crlf\r\n"),
        Ok("one crlf".to_owned())
    );
    assert_eq!(
        parse_secret_contents(&[0xff, 0xfe]),
        Err(SecretError::NotUtf8)
    );
    assert_eq!(
        parse_secret_contents(&vec![b'a'; MAX_SECRET_FILE_BYTES + 1]),
        Err(SecretError::TooLarge)
    );
    assert_eq!(
        parse_secret_contents(&vec![b'a'; MAX_SECRET_FILE_BYTES]),
        Ok("a".repeat(MAX_SECRET_FILE_BYTES))
    );
}

#[test]
fn a_relative_missing_or_empty_secret_file_is_refused() {
    let relative = config_with_files(&[("QR_DATABASE_URL_FILE", "run/secrets/database_url")], &[])
        .expect_err("a relative path");
    assert!(relative.to_string().contains("absolute path"), "{relative}");

    let missing = config_with_files(&[("QR_DATABASE_URL_FILE", "/run/secrets/missing")], &[])
        .expect_err("a missing file");
    assert!(missing.to_string().contains("cannot be read"), "{missing}");

    let empty = config_with_files(
        &[("QR_DATABASE_URL_FILE", "/run/secrets/empty")],
        &[("/run/secrets/empty", "\n")],
    )
    .expect_err("an empty file");
    assert!(empty.to_string().contains("must not be empty"), "{empty}");
}

#[test]
fn the_bearer_token_is_validated_and_its_file_is_recorded() {
    let config = config_with_files(
        &[("QR_BEARER_TOKEN_FILE", "/run/secrets/bearer_token")],
        &[("/run/secrets/bearer_token", SENTINEL_TOKEN)],
    )
    .expect("config");
    assert_eq!(config.bearer_token.as_deref(), Some(SENTINEL_TOKEN));
    assert_eq!(
        config.bearer_token_file.as_deref(),
        Some(Path::new("/run/secrets/bearer_token"))
    );

    let short = config_with_files(
        &[("QR_BEARER_TOKEN_FILE", "/run/secrets/bearer_token")],
        &[("/run/secrets/bearer_token", "short")],
    )
    .expect_err("a short token");
    assert!(short.to_string().contains("at least 32"), "{short}");
    assert!(!short.to_string().contains("short"), "{short}");
}

#[test]
fn the_token_reload_interval_is_bounded() {
    assert_eq!(
        Config::from_map(&BTreeMap::new())
            .expect("config")
            .bearer_token_reload,
        Duration::from_secs(5)
    );
    assert_eq!(
        Config::from_map(&values(&[("QR_TOKEN_RELOAD_SECONDS", "9")]))
            .expect("config")
            .bearer_token_reload,
        Duration::from_secs(9)
    );
    for invalid in ["0", "3601", "-1", "fast"] {
        let error = Config::from_map(&values(&[("QR_TOKEN_RELOAD_SECONDS", invalid)]))
            .expect_err("an invalid reload interval must be refused");
        assert!(
            error.to_string().contains("QR_TOKEN_RELOAD_SECONDS"),
            "{error}"
        );
    }
}

#[test]
fn an_object_store_url_with_userinfo_is_refused_without_echoing_it() {
    let error = Config::from_map(&values(&[(
        "QR_OBJECT_STORE_URL",
        "http://access:sentinel@minio:9000",
    )]))
    .expect_err("userinfo must be refused");
    let rendered = error.to_string();
    assert!(rendered.contains("QR_OBJECT_STORE_URL"), "{rendered}");
    assert!(rendered.contains("userinfo"), "{rendered}");
    assert!(!rendered.contains("sentinel"), "{rendered}");
}

#[test]
fn the_object_store_credentials_are_a_pair() {
    let error = config_with_files(
        &[(
            "QR_OBJECT_STORE_ACCESS_KEY_FILE",
            "/run/secrets/object_store_access_key",
        )],
        &[("/run/secrets/object_store_access_key", "quantrule")],
    )
    .expect_err("a lone access key");
    assert!(
        error
            .to_string()
            .contains("QR_OBJECT_STORE_SECRET_KEY_FILE"),
        "{error}"
    );

    let config = config_with_files(
        &[
            (
                "QR_OBJECT_STORE_ACCESS_KEY_FILE",
                "/run/secrets/object_store_access_key",
            ),
            (
                "QR_OBJECT_STORE_SECRET_KEY_FILE",
                "/run/secrets/object_store_secret_key",
            ),
        ],
        &[
            ("/run/secrets/object_store_access_key", "quantrule"),
            ("/run/secrets/object_store_secret_key", "sentinel-secret"),
        ],
    )
    .expect("both credential files");
    assert_eq!(config.object_store_access_key.as_deref(), Some("quantrule"));
    assert_eq!(
        config.object_store_secret_key.as_deref(),
        Some("sentinel-secret")
    );
}

#[test]
fn the_queue_generation_defaults_to_one_and_rejects_zero() {
    assert_eq!(
        Config::from_map(&BTreeMap::new())
            .expect("config")
            .queue_generation,
        DEFAULT_QUEUE_GENERATION
    );
    assert_eq!(
        Config::from_map(&values(&[("QR_QUEUE_GENERATION", "2")]))
            .expect("config")
            .queue_generation,
        2
    );
    for invalid in ["0", "-1", "one"] {
        let error = Config::from_map(&values(&[("QR_QUEUE_GENERATION", invalid)]))
            .expect_err("an invalid generation must be refused");
        assert!(error.to_string().contains("QR_QUEUE_GENERATION"), "{error}");
    }
}

#[test]
fn the_migration_only_argument_is_recognized() {
    assert!(migrate_only_in(
        [OsString::from("--migrate-only")].into_iter()
    ));
    assert!(!migrate_only_in([OsString::from("--serve")].into_iter()));
    assert!(!migrate_only_in(std::iter::empty()));
}

#[test]
fn the_metrics_bind_is_optional_and_validated() {
    assert_eq!(
        Config::from_map(&BTreeMap::new())
            .expect("config")
            .metrics_bind,
        None
    );
    assert_eq!(
        Config::from_map(&values(&[("QR_METRICS_BIND", "")]))
            .expect("config")
            .metrics_bind,
        None
    );
    assert_eq!(
        Config::from_map(&values(&[("QR_METRICS_BIND", "127.0.0.1:9090")]))
            .expect("config")
            .metrics_bind,
        Some(SocketAddr::from(([127, 0, 0, 1], 9090)))
    );
    let error = Config::from_map(&values(&[("QR_METRICS_BIND", "metrics:9090")]))
        .expect_err("a hostname is not an IP:port");
    assert!(error.to_string().contains("QR_METRICS_BIND"), "{error}");
}

#[test]
fn a_private_bind_is_required_for_metrics_when_internal() {
    let error = Config::from_map(&values(&[
        ("QR_INTERNAL_ONLY", "1"),
        ("QR_BIND", "10.0.0.5:8080"),
        ("QR_METRICS_BIND", "29.0.0.5:9090"),
    ]))
    .expect_err("a public metrics bind must be refused");
    assert!(error.to_string().contains("QR_METRICS_BIND"), "{error}");

    let config = Config::from_map(&values(&[
        ("QR_INTERNAL_ONLY", "1"),
        ("QR_BIND", "10.0.0.5:8080"),
        ("QR_METRICS_BIND", "10.0.0.5:9090"),
    ]))
    .expect("a private metrics bind");
    assert_eq!(
        config.metrics_bind,
        Some(SocketAddr::from(([10, 0, 0, 5], 9090)))
    );
}

#[test]
fn the_drain_deadlines_default_and_are_bounded() {
    let config = Config::from_map(&BTreeMap::new()).expect("config");
    assert_eq!(config.drain_timeout_http, Duration::from_secs(20));
    assert_eq!(config.drain_timeout_work, Duration::from_secs(25));

    let config = Config::from_map(&values(&[
        ("QR_DRAIN_TIMEOUT_HTTP", "7"),
        ("QR_DRAIN_TIMEOUT_WORK", "300"),
    ]))
    .expect("config");
    assert_eq!(config.drain_timeout_http, Duration::from_secs(7));
    assert_eq!(config.drain_timeout_work, Duration::from_secs(300));

    for invalid in ["0", "301", "-1", "fast"] {
        let error = Config::from_map(&values(&[("QR_DRAIN_TIMEOUT_HTTP", invalid)]))
            .expect_err("an invalid HTTP drain deadline must be refused");
        assert!(
            error.to_string().contains("QR_DRAIN_TIMEOUT_HTTP"),
            "{error}"
        );
        let error = Config::from_map(&values(&[("QR_DRAIN_TIMEOUT_WORK", invalid)]))
            .expect_err("an invalid worker drain deadline must be refused");
        assert!(
            error.to_string().contains("QR_DRAIN_TIMEOUT_WORK"),
            "{error}"
        );
    }
}

#[test]
fn a_tls_database_url_requires_the_ca_and_verify_full() {
    let config = config_with_files(
        &[
            ("QR_DATABASE_URL_FILE", "/run/secrets/database_url"),
            ("QR_DATABASE_TLS_CA_PATH", "/run/pki/ca.crt"),
        ],
        &[(
            "/run/secrets/database_url",
            "postgres://qr_gateway_app:secret@postgres:5432/qr_gateway?sslmode=verify-full",
        )],
    )
    .expect("TLS config");
    assert_eq!(
        config.database_tls_ca,
        Some(PathBuf::from("/run/pki/ca.crt"))
    );
    assert!(format!("{config:?}").contains("/run/pki/ca.crt"));

    for url in [
        "postgres://qr_gateway_app:secret@postgres:5432/qr_gateway",
        "postgres://qr_gateway_app:secret@postgres:5432/qr_gateway?sslmode=require",
        "postgres://qr_gateway_app:secret@postgres:5432/qr_gateway?sslmode=verify-ca",
    ] {
        let error = config_with_files(
            &[
                ("QR_DATABASE_URL_FILE", "/run/secrets/database_url"),
                ("QR_DATABASE_TLS_CA_PATH", "/run/pki/ca.crt"),
            ],
            &[("/run/secrets/database_url", url)],
        )
        .expect_err("a TLS endpoint without verify-full");
        assert!(
            error.to_string().contains("QR_DATABASE_URL_FILE"),
            "{error}"
        );
    }

    let error = config_with_files(
        &[("QR_DATABASE_URL_FILE", "/run/secrets/database_url")],
        &[(
            "/run/secrets/database_url",
            "postgres://qr_gateway_app:secret@postgres:5432/qr_gateway?sslmode=verify-full",
        )],
    )
    .expect_err("verify-full without a CA");
    assert!(
        error.to_string().contains("QR_DATABASE_TLS_CA_PATH"),
        "{error}"
    );
}

#[test]
fn a_tls_queue_and_object_store_pair_with_their_ca_paths() {
    let config = config_with_files(
        &[
            ("QR_QUEUE_URL_FILE", "/run/secrets/queue_url"),
            ("QR_QUEUE_TLS_CA_PATH", "/run/pki/ca.crt"),
            ("QR_OBJECT_STORE_TLS_CA_PATH", "/run/pki/ca.crt"),
            ("QR_OBJECT_STORE_URL", "https://minio:9000"),
        ],
        &[(
            "/run/secrets/queue_url",
            "amqps://quantrule:secret@rabbitmq:5671/%2f",
        )],
    )
    .expect("TLS queue");
    assert_eq!(config.queue_tls_ca, Some(PathBuf::from("/run/pki/ca.crt")));
    assert_eq!(
        config.object_store_tls_ca,
        Some(PathBuf::from("/run/pki/ca.crt"))
    );

    let error = config_with_files(
        &[
            ("QR_QUEUE_URL_FILE", "/run/secrets/queue_url"),
            ("QR_QUEUE_TLS_CA_PATH", "/run/pki/ca.crt"),
        ],
        &[(
            "/run/secrets/queue_url",
            "amqp://quantrule:secret@rabbitmq:5672/%2f",
        )],
    )
    .expect_err("amqp with a CA");
    assert!(error.to_string().contains("amqps"), "{error}");

    let error = config_with_files(&[("QR_OBJECT_STORE_TLS_CA_PATH", "pki/ca.crt")], &[])
        .expect_err("a relative CA path");
    assert!(error.to_string().contains("absolute"), "{error}");
}

#[test]
fn a_manually_built_config_cannot_bypass_the_metrics_bind_guard() {
    let config = Config {
        internal_only: true,
        bind: "10.0.0.5:8080".to_owned(),
        metrics_bind: Some(SocketAddr::from(([29, 0, 0, 5], 9090))),
        ..Config::default()
    };
    let error = config
        .check_internal_bind()
        .expect_err("public metrics bind");
    assert!(error.to_string().contains("QR_METRICS_BIND"), "{error}");
}
