//! Content-addressed identity.
//!
//! A digest is the only identity the distributed system trusts. A run, a
//! dataset, and an artifact are named by the bytes they contain, so two
//! identical inputs cannot produce two different names.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// The prefix every digest carries when written as a string.
pub const PREFIX: &str = "sha256:";

/// A SHA-256 content digest in lowercase hexadecimal.
///
/// The wire form is `sha256:<64 hex characters>`. Parsing accepts the bare hex
/// form and normalizes it, because a hand-written config should not have to
/// remember the prefix.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest(String);

/// Why a digest could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigestError {
    /// The value was empty.
    Empty,
    /// The value carried a prefix other than `sha256:`.
    UnknownAlgorithm,
    /// The hexadecimal part was not 64 characters.
    WrongLength,
    /// The hexadecimal part contained a character outside `0-9a-f`.
    NotHex,
}

impl std::fmt::Display for DigestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Empty => "a digest cannot be empty",
            Self::UnknownAlgorithm => "a digest must start with sha256:",
            Self::WrongLength => "a sha256 digest is 64 hexadecimal characters",
            Self::NotHex => "a digest contains a non-hexadecimal character",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for DigestError {}

impl Digest {
    /// Parse a digest from its wire form.
    ///
    /// # Errors
    ///
    /// Returns [`DigestError`] when the value is empty, carries another
    /// algorithm, is the wrong length, or is not lowercase hexadecimal.
    pub fn parse(value: &str) -> Result<Self, DigestError> {
        if value.is_empty() {
            return Err(DigestError::Empty);
        }
        let hex = match value.strip_prefix(PREFIX) {
            Some(rest) => rest,
            None => {
                if value.contains(':') {
                    return Err(DigestError::UnknownAlgorithm);
                }
                value
            }
        };
        if hex.len() != 64 {
            return Err(DigestError::WrongLength);
        }
        if !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(DigestError::NotHex);
        }
        Ok(Self(hex.to_owned()))
    }

    /// The bare lowercase hexadecimal form.
    #[must_use]
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// The wire form, including the `sha256:` prefix.
    #[must_use]
    pub fn to_wire(&self) -> String {
        format!("{PREFIX}{}", self.0)
    }

    /// The content digest of a byte slice.
    #[must_use]
    pub fn sha256(bytes: &[u8]) -> Self {
        Self(crate::sha256::sha256_hex(bytes))
    }

    /// The first eight characters, for a summary line.
    #[must_use]
    pub fn short(&self) -> &str {
        &self.0[..8]
    }
}

impl std::fmt::Display for Digest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(PREFIX)?;
        formatter.write_str(&self.0)
    }
}

impl std::str::FromStr for Digest {
    type Err = DigestError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl Serialize for Digest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_wire())
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn parses_the_wire_form() {
        let digest = Digest::parse(&format!("sha256:{HEX}")).expect("wire form");
        assert_eq!(digest.as_hex(), HEX);
        assert_eq!(digest.to_wire(), format!("sha256:{HEX}"));
    }

    #[test]
    fn normalizes_a_bare_hex_value() {
        let digest = Digest::parse(HEX).expect("bare form");
        assert_eq!(digest.to_string(), format!("sha256:{HEX}"));
    }

    #[test]
    fn rejects_a_foreign_algorithm() {
        assert_eq!(
            Digest::parse(&format!("md5:{HEX}")),
            Err(DigestError::UnknownAlgorithm)
        );
    }

    #[test]
    fn rejects_the_wrong_length_and_case() {
        assert_eq!(Digest::parse("sha256:abc"), Err(DigestError::WrongLength));
        let upper = "A".repeat(64);
        assert_eq!(Digest::parse(&upper), Err(DigestError::NotHex));
    }

    #[test]
    fn rejects_an_empty_value() {
        assert_eq!(Digest::parse(""), Err(DigestError::Empty));
    }

    #[test]
    fn round_trips_through_json() {
        let digest = Digest::parse(HEX).expect("fixture");
        let text = serde_json::to_string(&digest).expect("encode");
        assert_eq!(text, format!("\"sha256:{HEX}\""));
        let back: Digest = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, digest);
    }

    #[test]
    fn short_form_is_eight_characters() {
        let digest = Digest::parse(HEX).expect("fixture");
        assert_eq!(digest.short(), "01234567");
    }
}
