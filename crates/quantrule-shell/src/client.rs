//! The one HTTP transport of the shell.
//!
//! Every command calls one gateway path through this module. The shell never
//! reaches an engine, opens a database, or computes a measurement; it builds a
//! request, forwards the profile header, and returns the response bytes.
//!
//! A non-2xx response is not an I/O failure: the shared envelope is parsed and
//! its stable code is preserved. A body that does not match the contract is a
//! decode failure, named with the path that produced it.

use std::time::Duration;

use quantrule_api::{PROFILE_HEADER, ProfileId};

use crate::error::RunError;

/// The gateway base used when `QUANTRULE_API_URL` is unset.
pub(crate) const DEFAULT_API_URL: &str = "http://127.0.0.1:8080/api/v1";

/// One response, before the command decodes it.
pub(crate) struct Response {
    /// The HTTP status.
    pub(crate) status: u16,
    /// The response body.
    pub(crate) body: Vec<u8>,
}

/// A gateway client bound to one base URL and one profile.
pub(crate) struct Client {
    agent: ureq::Agent,
    base_url: String,
    profile: ProfileId,
}

impl Client {
    /// Build a client for one profile.
    ///
    /// # Errors
    ///
    /// The agent construction is infallible; the result is kept so the caller
    /// has one shape for every setup failure.
    pub(crate) fn new(base_url: String, profile: ProfileId) -> Result<Self, RunError> {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(120)))
            .build();
        Ok(Self {
            agent: ureq::Agent::new_with_config(config),
            base_url: base_url.trim_end_matches('/').to_owned(),
            profile,
        })
    }

    /// GET one path.
    ///
    /// # Errors
    ///
    /// Returns [`RunError::Transport`] when the exchange fails and
    /// [`RunError::Api`] when the gateway refuses the call.
    pub(crate) fn get(&self, path: &str) -> Result<Response, RunError> {
        let url = self.url(path);
        let response = self
            .agent
            .get(&url)
            .header(PROFILE_HEADER, self.profile.as_str())
            .header("accept", "application/json")
            .call()
            .map_err(|error| RunError::Transport(error.to_string()))?;
        read(response, path)
    }

    /// POST a JSON body.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Client::get`].
    pub(crate) fn post_json(&self, path: &str, body: &str) -> Result<Response, RunError> {
        let url = self.url(path);
        let response = self
            .agent
            .post(&url)
            .header(PROFILE_HEADER, self.profile.as_str())
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .send(body)
            .map_err(|error| RunError::Transport(error.to_string()))?;
        read(response, path)
    }

    /// POST a CSV body.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Client::get`].
    pub(crate) fn post_csv(&self, path: &str, body: Vec<u8>) -> Result<Response, RunError> {
        let url = self.url(path);
        let response = self
            .agent
            .post(&url)
            .header(PROFILE_HEADER, self.profile.as_str())
            .header("accept", "application/json")
            .header("content-type", "text/csv")
            .send(body)
            .map_err(|error| RunError::Transport(error.to_string()))?;
        read(response, path)
    }

    /// POST an empty body.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Client::get`].
    pub(crate) fn post_empty(&self, path: &str) -> Result<Response, RunError> {
        let url = self.url(path);
        let response = self
            .agent
            .post(&url)
            .header(PROFILE_HEADER, self.profile.as_str())
            .header("accept", "application/json")
            .send_empty()
            .map_err(|error| RunError::Transport(error.to_string()))?;
        read(response, path)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
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

fn read(response: ureq::http::Response<ureq::Body>, path: &str) -> Result<Response, RunError> {
    let status = response.status().as_u16();
    let body = response
        .into_body()
        .read_to_vec()
        .map_err(|error| RunError::Transport(error.to_string()))?;
    if (200..300).contains(&status) {
        return Ok(Response { status, body });
    }

    let value: Option<serde_json::Value> = serde_json::from_slice(&body).ok();
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
        .unwrap_or_else(|| format!("{path} answered with HTTP {status}"));
    Err(RunError::api(code, &message))
}

#[cfg(test)]
mod tests {
    use super::encode;

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
