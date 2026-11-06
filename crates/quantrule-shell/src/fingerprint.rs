//! The stable FNV-1a content fingerprint shared by stage provenance.
//!
//! The fingerprint is an identity aid, not a security primitive. It is stable
//! for the same bytes on every platform, so a later run can be shown to have
//! consumed the same input file. It must not be used to authenticate bytes that
//! an adversary controls.

/// Returns the 16-digit lowercase FNV-1a fingerprint of `bytes`.
#[must_use]
pub(crate) fn fnv1a(bytes: &[u8]) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::fnv1a;

    #[test]
    fn the_fingerprint_is_stable_and_content_sensitive() {
        assert_eq!(fnv1a(b"abc"), fnv1a(b"abc"));
        assert_ne!(fnv1a(b"abc"), fnv1a(b"abd"));
        assert_eq!(fnv1a(b"abc").len(), 16);
        assert!(fnv1a(b"abc").chars().all(|value| value.is_ascii_hexdigit()));
    }
}
