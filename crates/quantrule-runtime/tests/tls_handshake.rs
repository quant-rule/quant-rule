//! In-memory TLS handshake fixtures for the object-store connector.
//!
//! The private CA model is the contract: the client trusts one bundled CA and
//! rejects a wrong CA, a hostname mismatch and an expired leaf. The fixture
//! generates every certificate in memory with `rcgen`; no private key is read
//! from the repository. The server side is a tiny TLS listener that answers one
//! S3 `HEAD` with `200`, so the test exercises the real Smithy connector and
//! trust store rather than a mock.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use quantrule_runtime::adapters::s3::S3Store;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, IsCa, Issuer, KeyPair, KeyUsagePurpose,
    date_time_ymd,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

struct Fixture {
    ca_pem: String,
    ca_params: CertificateParams,
    ca_key: KeyPair,
}

impl Fixture {
    fn new() -> Self {
        let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();
        let mut ca_params =
            CertificateParams::new(vec!["quantrule-test-ca".to_owned()]).expect("CA parameters");
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let ca_key = KeyPair::generate().expect("CA key");
        let ca_cert = ca_params.self_signed(&ca_key).expect("CA certificate");
        Self {
            ca_pem: ca_cert.pem(),
            ca_params,
            ca_key,
        }
    }

    fn leaf(
        &self,
        names: &[&str],
        not_before: (i32, u8, u8),
        not_after: (i32, u8, u8),
    ) -> (Certificate, KeyPair) {
        let mut params = CertificateParams::new(
            names
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>(),
        )
        .expect("leaf parameters");
        params.not_before = date_time_ymd(not_before.0, not_before.1, not_before.2);
        params.not_after = date_time_ymd(not_after.0, not_after.1, not_after.2);
        params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
        let key = KeyPair::generate().expect("leaf key");
        let issuer = Issuer::from_params(&self.ca_params, &self.ca_key);
        let cert = params.signed_by(&key, &issuer).expect("signed leaf");
        (cert, key)
    }
}

fn ca_file(pem: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "qr-tls-ca-{}-{}.crt",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, pem).expect("CA fixture");
    path
}

async fn serve(cert: Certificate, key: KeyPair) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let certificate = CertificateDer::from(cert.der().to_vec());
    let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der()));
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certificate], private_key)
        .expect("server config");
    let acceptor = TlsAcceptor::from(std::sync::Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let address = listener.local_addr().expect("address");
    let task = tokio::spawn(async move {
        loop {
            let Ok((stream, _peer)) = listener.accept().await else {
                return;
            };
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                let Ok(mut stream) = acceptor.accept(stream).await else {
                    return;
                };
                let mut buffer = [0_u8; 2048];
                let _ = stream.read(&mut buffer).await;
                let _ = stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .await;
            });
        }
    });
    (address, task)
}

async fn check(address: SocketAddr, ca: &Path) -> Result<(), String> {
    let store = S3Store::new(
        &format!("https://{address}"),
        "quantrule",
        "quantrule",
        "fixture-secret",
        Some(ca),
    )
    .map_err(|error| error.to_string())?;
    tokio::time::timeout(Duration::from_secs(10), store.check())
        .await
        .map_err(|_| "the handshake timed out".to_owned())?
        .map_err(|error| error.to_string())
}

#[tokio::test]
async fn the_private_ca_completes_a_handshake() {
    let fixture = Fixture::new();
    let ca = ca_file(&fixture.ca_pem);
    let (cert, key) = fixture.leaf(&["127.0.0.1"], (2024, 1, 1), (2030, 1, 1));
    let (address, server) = serve(cert, key).await;
    check(address, &ca)
        .await
        .expect("the private CA is trusted");
    server.abort();
    let _ = std::fs::remove_file(ca);
}

#[tokio::test]
async fn a_wrong_ca_is_rejected() {
    let trusted = Fixture::new();
    let other = Fixture::new();
    let ca = ca_file(&trusted.ca_pem);
    let (cert, key) = other.leaf(&["127.0.0.1"], (2024, 1, 1), (2030, 1, 1));
    let (address, server) = serve(cert, key).await;
    let error = check(address, &ca).await.expect_err("a foreign CA");
    assert!(!error.is_empty(), "the handshake failed without a reason");
    server.abort();
    let _ = std::fs::remove_file(ca);
}

#[tokio::test]
async fn a_hostname_mismatch_is_rejected() {
    let fixture = Fixture::new();
    let ca = ca_file(&fixture.ca_pem);
    let (cert, key) = fixture.leaf(&["wrong.example"], (2024, 1, 1), (2030, 1, 1));
    let (address, server) = serve(cert, key).await;
    let error = check(address, &ca).await.expect_err("a hostname mismatch");
    assert!(!error.is_empty(), "the handshake failed without a reason");
    server.abort();
    let _ = std::fs::remove_file(ca);
}

#[tokio::test]
async fn an_expired_certificate_is_rejected() {
    let fixture = Fixture::new();
    let ca = ca_file(&fixture.ca_pem);
    let (cert, key) = fixture.leaf(&["127.0.0.1"], (2020, 1, 1), (2021, 1, 1));
    let (address, server) = serve(cert, key).await;
    let error = check(address, &ca)
        .await
        .expect_err("an expired certificate");
    assert!(!error.is_empty(), "the handshake failed without a reason");
    server.abort();
    let _ = std::fs::remove_file(ca);
}
