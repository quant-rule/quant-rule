//! Transport TLS configuration.
//!
//! The deployed profile makes TLS mandatory for PostgreSQL, RabbitMQ and the
//! object store. The CA path is not a secret — it is a file the process reads
//! before a handshake — so it stays a plain `QR_*_TLS_CA_PATH` value. This
//! module owns the pairing rule: a TLS endpoint must name a CA, a CA must not
//! sit beside a plaintext endpoint, and the database never accepts a mode that
//! skips hostname verification.
//!
//! Internal HTTP is deliberately not here. The private overlay carries it and
//! the residual risk is recorded in the TLS contract; adding an HTTPS connector
//! would be a different decision.

use std::path::{Path, PathBuf};

use super::ConfigError;

/// The CA path for the PostgreSQL application and owner URLs.
pub const DATABASE_CA: &str = "QR_DATABASE_TLS_CA_PATH";
/// The CA path for the AMQP URL.
pub const QUEUE_CA: &str = "QR_QUEUE_TLS_CA_PATH";
/// The CA path for the S3 endpoint.
pub const OBJECT_STORE_CA: &str = "QR_OBJECT_STORE_TLS_CA_PATH";

/// Parse one CA path under the same absolute-path rule as a secret file.
///
/// The path is not a secret and is printed in `Config` debug output. It must
/// be absolute so a service never reads a CA relative to an unpredictable
/// working directory. An empty value is rejected rather than treated as unset.
pub(super) fn parse_ca_path(name: &'static str, value: &str) -> Result<PathBuf, ConfigError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ConfigError::Invalid {
            name,
            value: value.to_owned(),
            reason: "must not be empty",
        });
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(ConfigError::Invalid {
            name,
            value: value.to_owned(),
            reason: "must be an absolute path",
        });
    }
    Ok(path)
}

/// Refuse a PostgreSQL URL whose TLS mode skips verification or disagrees with
/// the CA path.
///
/// `sslmode=verify-full` is the only accepted TLS mode. `prefer`, `require` and
/// `verify-ca` are refused because they either fall back to plaintext or omit
/// the hostname check, and a CA path without a TLS mode is refused because it
/// would leave the connection plaintext while looking protected.
pub(super) fn validate_database(url: &str, ca: Option<&Path>) -> Result<(), ConfigError> {
    let mode = ssl_mode(url);
    match (ca, mode.as_deref()) {
        (Some(_), Some("verify-full")) => Ok(()),
        (Some(_), Some(mode)) => Err(ConfigError::Secret {
            name: "QR_DATABASE_URL_FILE",
            reason: match mode {
                "prefer" | "require" | "verify-ca" => {
                    "must use sslmode=verify-full when a TLS CA is configured"
                }
                _ => "has an unknown sslmode",
            },
        }),
        (Some(_), None) => Err(ConfigError::Secret {
            name: "QR_DATABASE_URL_FILE",
            reason: "must use sslmode=verify-full when a TLS CA is configured",
        }),
        (None, Some("disable")) | (None, None) => Ok(()),
        (None, Some(mode))
            if mode.starts_with("verify") || mode == "require" || mode == "prefer" =>
        {
            Err(ConfigError::Secret {
                name: "QR_DATABASE_URL_FILE",
                reason: "a TLS sslmode requires QR_DATABASE_TLS_CA_PATH",
            })
        }
        (None, Some(_)) => Err(ConfigError::Secret {
            name: "QR_DATABASE_URL_FILE",
            reason: "has an unknown sslmode",
        }),
    }
}

/// Refuse an AMQP URL whose scheme disagrees with the CA path.
///
/// `amqps://` is the only TLS scheme the AMQP client upgrades. A CA beside
/// `amqp://` is a misconfiguration that would otherwise send credentials in
/// plaintext.
pub(super) fn validate_queue(url: &str, ca: Option<&Path>) -> Result<(), ConfigError> {
    let tls = url.starts_with("amqps://");
    let plain = url.starts_with("amqp://");
    match (ca, tls, plain) {
        (Some(_), true, _) => Ok(()),
        (None, false, true) => Ok(()),
        (Some(_), false, _) => Err(ConfigError::Secret {
            name: "QR_QUEUE_URL_FILE",
            reason: "must use amqps:// when QR_QUEUE_TLS_CA_PATH is configured",
        }),
        (None, true, _) => Err(ConfigError::Secret {
            name: "QR_QUEUE_URL_FILE",
            reason: "amqps:// requires QR_QUEUE_TLS_CA_PATH",
        }),
        (_, false, false) => Err(ConfigError::Secret {
            name: "QR_QUEUE_URL_FILE",
            reason: "must use an amqp:// or amqps:// scheme",
        }),
    }
}

/// Refuse an object-store URL whose scheme disagrees with the CA path.
///
/// The endpoint carries no credentials, so the ordinary invalid-value error is
/// safe and keeps the full URL in the message.
pub(super) fn validate_object_store(url: &str, ca: Option<&Path>) -> Result<(), ConfigError> {
    let tls = url.starts_with("https://");
    let plain = url.starts_with("http://");
    match (ca, tls, plain) {
        (Some(_), true, _) | (None, false, true) => Ok(()),
        (Some(_), false, _) => Err(ConfigError::Invalid {
            name: "QR_OBJECT_STORE_URL",
            value: url.to_owned(),
            reason: "must use https:// when QR_OBJECT_STORE_TLS_CA_PATH is configured",
        }),
        (None, true, _) => Err(ConfigError::Invalid {
            name: "QR_OBJECT_STORE_URL",
            value: url.to_owned(),
            reason: "https:// requires QR_OBJECT_STORE_TLS_CA_PATH",
        }),
        (_, false, false) => Err(ConfigError::Invalid {
            name: "QR_OBJECT_STORE_URL",
            value: url.to_owned(),
            reason: "must use an http:// or https:// scheme",
        }),
    }
}

/// Read `sslmode` from a URL query without decoding or re-serialising it.
///
/// A duplicate mode is ambiguous and refused by the caller as an unknown value;
/// this helper returns the first one so the error stays stable.
fn ssl_mode(url: &str) -> Option<String> {
    let (_, query) = url.split_once('?')?;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == "sslmode").then(|| value.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ca() -> Option<&'static Path> {
        Some(Path::new("/run/pki/ca.crt"))
    }

    #[test]
    fn a_database_url_needs_verify_full_exactly_when_a_ca_is_set() {
        for url in [
            "postgres://qr_gateway_app:secret@postgres:5432/qr_gateway?sslmode=verify-full",
            "postgresql://qr_gateway_app:secret@postgres:5432/qr_gateway?sslmode=verify-full",
        ] {
            validate_database(url, ca()).expect(url);
        }
        for url in [
            "postgres://quantrule@postgres:5432/qr_gateway",
            "postgres://quantrule@postgres:5432/qr_gateway?sslmode=prefer",
            "postgres://quantrule@postgres:5432/qr_gateway?sslmode=require",
            "postgres://quantrule@postgres:5432/qr_gateway?sslmode=verify-ca",
            "postgres://quantrule@postgres:5432/qr_gateway?sslmode=disable",
        ] {
            assert!(validate_database(url, ca()).is_err(), "{url} was accepted");
        }
        validate_database(
            "postgres://quantrule@postgres:5432/qr_gateway?sslmode=disable",
            None,
        )
        .expect("plaintext is allowed only without a CA");
        assert!(
            validate_database(
                "postgres://quantrule@postgres:5432/qr_gateway?sslmode=verify-full",
                None
            )
            .is_err(),
            "a TLS mode without a CA must fail closed"
        );
    }

    #[test]
    fn a_queue_scheme_must_match_the_ca() {
        validate_queue("amqps://quantrule:secret@rabbitmq:5671/%2f", ca()).expect("amqps");
        validate_queue("amqp://quantrule:secret@rabbitmq:5672/%2f", None).expect("amqp");
        assert!(validate_queue("amqp://quantrule:secret@rabbitmq:5672/%2f", ca()).is_err());
        assert!(validate_queue("amqps://quantrule:secret@rabbitmq:5671/%2f", None).is_err());
        assert!(validate_queue("http://rabbitmq:5672", None).is_err());
    }

    #[test]
    fn an_object_store_scheme_must_match_the_ca() {
        validate_object_store("https://minio:9000", ca()).expect("https");
        validate_object_store("http://minio:9000", None).expect("http");
        assert!(validate_object_store("http://minio:9000", ca()).is_err());
        assert!(validate_object_store("https://minio:9000", None).is_err());
        assert!(validate_object_store("s3://minio:9000", None).is_err());
    }

    #[test]
    fn a_ca_path_must_be_absolute_and_not_empty() {
        assert_eq!(
            parse_ca_path(DATABASE_CA, "/run/pki/ca.crt").expect("absolute"),
            PathBuf::from("/run/pki/ca.crt")
        );
        for value in ["", "   ", "pki/ca.crt", "./ca.crt"] {
            assert!(parse_ca_path(DATABASE_CA, value).is_err(), "{value:?}");
        }
    }
}
