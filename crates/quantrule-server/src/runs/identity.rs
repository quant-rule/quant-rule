//! The content-derived run identity.

use quantrule_api::{Digest, RunId};

/// Derive a run id from the four inputs the contract names.
///
/// The fields are newline-delimited and tagged with the identity version, so a
/// field boundary cannot be forged by concatenation.
#[must_use]
pub fn derive(
    data_version: &Digest,
    config_hash: &Digest,
    model_version: &str,
    image_digest: &Digest,
) -> RunId {
    let mut bytes = b"quantrule.run/v1\n".to_vec();
    for part in [
        data_version.to_string(),
        config_hash.to_string(),
        model_version.to_owned(),
        image_digest.to_string(),
    ] {
        bytes.extend_from_slice(part.as_bytes());
        bytes.push(b'\n');
    }
    RunId(Digest::sha256(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(fill: &[u8]) -> Digest {
        Digest::sha256(fill)
    }

    #[test]
    fn the_same_inputs_give_the_same_id() {
        let data = digest(b"data");
        let config = digest(b"config");
        let image = digest(b"image");
        assert_eq!(
            derive(&data, &config, "M1.0", &image),
            derive(&data, &config, "M1.0", &image)
        );
    }

    #[test]
    fn every_input_changes_the_id() {
        let data = digest(b"data");
        let config = digest(b"config");
        let image = digest(b"image");
        let base = derive(&data, &config, "M1.0", &image);
        assert_ne!(base, derive(&digest(b"other"), &config, "M1.0", &image));
        assert_ne!(base, derive(&data, &digest(b"other"), "M1.0", &image));
        assert_ne!(base, derive(&data, &config, "M2.0", &image));
        assert_ne!(base, derive(&data, &config, "M1.0", &digest(b"other")));
    }

    #[test]
    fn a_field_boundary_is_not_ambiguous() {
        let data = digest(b"data");
        let config = digest(b"config");
        let image = digest(b"image");
        assert_ne!(
            derive(&data, &config, "ab", &image),
            derive(&data, &config, "a\nb", &image)
        );
    }
}
