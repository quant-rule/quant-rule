//! The one HTTP transport of the shell.
//!
//! Every command calls one gateway path through this module. The shell never
//! reaches an engine, opens a database, or computes a measurement; it builds a
//! request, forwards the profile header, attaches the bearer token when one is
//! configured, and returns the response bytes.
//!
//! A `GET` is the only retryable method: it repeats a transport failure, `408`,
//! `429`, `502`, `503` and `504` under the bounded policy in [`crate::retry`].
//! A mutation, a streaming upload and an invalid response are single-attempt. A
//! non-2xx response is not an I/O failure: the shared envelope is parsed and its
//! stable code is preserved. After the final attempt the transport failure names
//! the attempt count and never the URL, token, or body.

use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use quantrule_api::{PROFILE_HEADER, ProfileId};
use ureq::tls::{Certificate, RootCerts, TlsConfig};

use crate::error::RunError;
use crate::retry;
use crate::token::BearerToken;

/// The environment variable naming a PEM CA bundle for a private deployment.
pub(crate) const CA_CERT_ENV: &str = "QUANTRULE_CA_CERT";

/// The gateway base used when `QUANTRULE_API_URL` is unset.
pub(crate) const DEFAULT_API_URL: &str = "http://127.0.0.1:8080/api/v1";

/// The longest one exchange may take, matching the agent default.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// One response, before the command decodes it.
pub(crate) struct Response {
    /// The HTTP status.
    pub(crate) status: u16,
    /// The response body.
    pub(crate) body: Vec<u8>,
}

/// A gateway client bound to one base URL, one profile, and one token.
pub(crate) struct Client {
    agent: ureq::Agent,
    base_url: String,
    profile: ProfileId,
    token: Option<BearerToken>,
}

impl Client {
    /// Build a client for one profile and an optional bearer token.
    ///
    /// # Errors
    ///
    /// The agent construction is infallible; the result is kept so the caller
    /// has one shape for every setup failure.
    pub(crate) fn new(
        base_url: String,
        profile: ProfileId,
        token: Option<BearerToken>,
    ) -> Result<Self, RunError> {
        let mut config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(REQUEST_TIMEOUT));
        if let Some(path) = std::env::var_os(CA_CERT_ENV) {
            let path = PathBuf::from(path);
            let certificate = load_ca(&path)?;
            config = config.tls_config(
                TlsConfig::builder()
                    .root_certs(RootCerts::from(vec![certificate]))
                    .build(),
            );
        }
        Ok(Self {
            agent: ureq::Agent::new_with_config(config.build()),
            base_url: base_url.trim_end_matches('/').to_owned(),
            profile,
            token,
        })
    }

    /// GET one path under the bounded retry policy.
    ///
    /// # Errors
    ///
    /// Returns [`RunError::Transport`] when every attempt fails and
    /// [`RunError::Api`] when the gateway refuses the call.
    pub(crate) fn get(&self, path: &str) -> Result<Response, RunError> {
        let url = self.url(path);
        let started = Instant::now();
        let mut last_attempt = 1;

        for attempt in 1..=retry::ATTEMPTS {
            last_attempt = attempt;
            let remaining = retry::TOTAL_DEADLINE.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                break;
            }
            let builder = self
                .agent
                .get(&url)
                .config()
                .timeout_global(Some(remaining.min(REQUEST_TIMEOUT)))
                .build();
            match exchange(self.common_headers(builder).call()) {
                Ok(response) if retry::is_retryable_status(response.status) => {}
                Ok(response) => return checked(response, path),
                Err(()) => {}
            }

            let Some(delay) = retry::delay_after(attempt) else {
                break;
            };
            let remaining = retry::TOTAL_DEADLINE.saturating_sub(started.elapsed());
            if delay >= remaining {
                break;
            }
            thread::sleep(delay);
        }

        Err(RunError::transport(last_attempt))
    }

    /// POST a JSON body.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Client::get`], with one attempt.
    pub(crate) fn post_json(&self, path: &str, body: &str) -> Result<Response, RunError> {
        self.post_bytes(path, "application/json", body.as_bytes().to_vec())
    }

    /// POST a JSON body read from a local file.
    ///
    /// The bytes are sent unchanged; the server canonicalizes and hashes them.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Client::get`], with one attempt.
    pub(crate) fn post_json_bytes(&self, path: &str, body: Vec<u8>) -> Result<Response, RunError> {
        self.post_bytes(path, "application/json", body)
    }

    /// POST one body with an explicit content type.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Client::get`], with one attempt.
    pub(crate) fn post_bytes(
        &self,
        path: &str,
        content_type: &str,
        body: Vec<u8>,
    ) -> Result<Response, RunError> {
        let url = self.url(path);
        let response = exchange(
            self.common_headers(self.agent.post(&url))
                .header("content-type", content_type)
                .send(body),
        )
        .map_err(|()| RunError::transport(1))?;
        checked(response, path)
    }

    /// POST a CSV body.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Client::get`], with one attempt.
    pub(crate) fn post_csv(&self, path: &str, body: Vec<u8>) -> Result<Response, RunError> {
        self.post_bytes(path, "text/csv", body)
    }

    /// POST an empty body.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Client::get`], with one attempt.
    pub(crate) fn post_empty(&self, path: &str) -> Result<Response, RunError> {
        let url = self.url(path);
        let response = exchange(self.common_headers(self.agent.post(&url)).send_empty())
            .map_err(|()| RunError::transport(1))?;
        checked(response, path)
    }

    /// DELETE one path, accepting a `204` with no body.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Client::get`], with one attempt.
    pub(crate) fn delete_empty(&self, path: &str) -> Result<Response, RunError> {
        let url = self.url(path);
        let response = exchange(self.common_headers(self.agent.delete(&url)).call())
            .map_err(|()| RunError::transport(1))?;
        checked(response, path)
    }

    /// Attach the headers every gateway call carries.
    fn common_headers<B>(&self, request: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        let request = request
            .header(PROFILE_HEADER, self.profile.as_str())
            .header("accept", "application/json");
        match &self.token {
            Some(token) => request.header("authorization", token.header()),
            None => request,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

/// Read one PEM CA bundle for a private deployment.
///
/// The default `ureq` roots are a fixed webpki set, so a private CA is not
/// trusted without this override. When the variable is unset the agent keeps
/// the default roots; when it is set only the named CA is trusted, exactly as
/// `qr-deploy check` trusts `endpoints.database.tls_ca`.
fn load_ca(path: &Path) -> Result<Certificate<'static>, RunError> {
    let pem = std::fs::read(path).map_err(|error| RunError::file(path.display(), error))?;
    Certificate::from_pem(&pem).map_err(|error| RunError::File {
        path: path.display().to_string(),
        message: format!("is not a PEM CA certificate: {error}"),
    })
}

/// Read one exchange into status and bytes.
///
/// A transport or body failure is `Err(())`: the caller retries a GET and turns
/// a mutation failure into the stable attempt-count error. The raw error is not
/// kept because it can carry a URL, a token, or a response body.
fn exchange(
    response: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<Response, ()> {
    let response = response.map_err(|_| ())?;
    let status = response.status().as_u16();
    let body = response.into_body().read_to_vec().map_err(|_| ())?;
    Ok(Response { status, body })
}

/// Percent-encode one path segment or query value.
///
/// The gateway accepts an identifier as one segment, so `sha256:` and a name
/// with a space must be encoded rather than pasted into the URL.
pub(crate) fn encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => {
                encoded.push('%');
                encoded.push(hex_digit(byte >> 4));
                encoded.push(hex_digit(byte & 0x0f));
            }
        }
    }
    encoded
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        _ => char::from(b'A' + (value - 10)),
    }
}

/// Preserve a 2xx, and turn a non-2xx into the shared envelope error.
fn checked(response: Response, path: &str) -> Result<Response, RunError> {
    if (200..300).contains(&response.status) {
        return Ok(response);
    }

    let value: Option<serde_json::Value> = serde_json::from_slice(&response.body).ok();
    let code = value
        .as_ref()
        .and_then(|value| value.pointer("/error/code"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("internal");
    let message = value
        .as_ref()
        .and_then(|value| value.pointer("/error/message"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{path} answered with HTTP {}", response.status));
    Err(RunError::api(code, &message))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{encode, load_ca};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_file(name: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "qr-shell-ca-{}-{}-{name}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, content).expect("temp file");
        path
    }

    #[test]
    fn a_missing_ca_file_is_a_local_error() {
        let path = std::env::temp_dir().join(format!(
            "qr-shell-missing-ca-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let error = load_ca(&path).expect_err("a missing CA");
        assert!(error.to_string().contains("cannot use"), "{}", error);
    }

    #[test]
    fn a_file_that_is_not_a_pem_certificate_is_refused() {
        let path = temp_file("not-a-cert", "not a certificate\n");
        let error = load_ca(&path).expect_err("a malformed CA");
        assert!(
            error.to_string().contains("PEM CA certificate"),
            "{}",
            error
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_digest_becomes_one_encoded_segment() {
        assert_eq!(encode("sha256:abc"), "sha256%3Aabc");
    }

    #[test]
    fn an_unreserved_value_is_unchanged() {
        assert_eq!(encode("cfg_01-alpha.json~"), "cfg_01-alpha.json~");
    }

    #[test]
    fn a_space_and_a_slash_are_encoded() {
        assert_eq!(encode("a b/c"), "a%20b%2Fc");
    }
}
