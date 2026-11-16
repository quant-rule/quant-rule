//! The desktop package identity.
//!
//! A packaged run must record which package executed it, because the run id is
//! derived from the image digest. The supervisor therefore hashes the six
//! installed sidecars and exports the result as `QR_IMAGE_DIGEST` to every
//! child, so `/health` and provenance agree with the package that is running.
//!
//! The hash is SHA-256 implemented here because the supervisor deliberately has
//! no dependencies: the boundary test proves it cannot link an engine, the
//! application service, or Tauri, and a digest crate would be the only
//! dependency in the binary. It is used for identity, never for authentication
//! or secrecy. The canonical form is the file name, a zero byte, the big-endian
//! byte length, and the bytes, in file-name order.

use std::fs;
use std::path::{Path, PathBuf};

/// The prefix every wire digest carries.
const PREFIX: &str = "sha256:";

/// The SHA-256 round constants from FIPS 180-4.
const ROUND: [u32; 64] = [
    0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4,
    0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe,
    0x9bdc_06a7, 0xc19b_f174, 0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc, 0x2de9_2c6f,
    0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da, 0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7,
    0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967, 0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc,
    0x5338_0d13, 0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85, 0xa2bf_e8a1, 0xa81a_664b,
    0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070, 0x19a4_c116,
    0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3,
    0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208, 0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7,
    0xc671_78f2,
];

/// A streaming SHA-256 state.
struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffered: usize,
    length: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a, 0x510e_527f, 0x9b05_688c,
                0x1f83_d9ab, 0x5be0_cd19,
            ],
            buffer: [0; 64],
            buffered: 0,
            length: 0,
        }
    }

    fn update(&mut self, mut bytes: &[u8]) {
        self.length = self.length.wrapping_add(bytes.len() as u64);
        if self.buffered > 0 {
            let take = (64 - self.buffered).min(bytes.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&bytes[..take]);
            self.buffered += take;
            bytes = &bytes[take..];
            if self.buffered < 64 {
                return;
            }
            let block = self.buffer;
            self.compress(&block);
            self.buffered = 0;
        }
        while bytes.len() >= 64 {
            let mut block = [0_u8; 64];
            block.copy_from_slice(&bytes[..64]);
            self.compress(&block);
            bytes = &bytes[64..];
        }
        if !bytes.is_empty() {
            self.buffer[..bytes.len()].copy_from_slice(bytes);
            self.buffered = bytes.len();
        }
    }

    fn finish(mut self) -> [u8; 32] {
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.buffered != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());
        let mut digest = [0_u8; 32];
        for (index, word) in self.state.iter().enumerate() {
            digest[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        digest
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut words = [0_u32; 64];
        for (index, chunk) in block.chunks_exact(4).enumerate() {
            words[index] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(ROUND[index])
                .wrapping_add(words[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in self
            .state
            .iter_mut()
            .zip([a, b, c, d, e, f, g, h].into_iter())
        {
            *slot = slot.wrapping_add(value);
        }
    }
}

/// The lowercase hexadecimal form of a digest.
fn hex(digest: &[u8; 32]) -> String {
    let mut text = String::with_capacity(64);
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

/// Hash the six installed sidecars into one package digest.
///
/// The files are hashed in file-name order, so the result does not depend on
/// the order the supervisor lists its services. The digest is stable for one
/// package and changes when any sidecar changes, which makes an upgraded
/// package a distinct execution under the run contract.
///
/// # Errors
///
/// Returns a message naming the file when the set does not hold six sidecars,
/// a path has no file name, or a sidecar cannot be read.
pub fn digest(programs: &[PathBuf]) -> Result<String, String> {
    if programs.len() != 6 {
        return Err(format!(
            "the package digest needs six sidecars, got {}",
            programs.len()
        ));
    }
    let mut files = Vec::with_capacity(programs.len());
    for program in programs {
        let name = program
            .file_name()
            .ok_or_else(|| format!("{} has no file name", program.display()))?
            .to_string_lossy()
            .into_owned();
        files.push((name, program));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    for (name, program) in files {
        let bytes = fs::read(program)
            .map_err(|error| format!("cannot read {} for the package digest: {error}", program.display()))?;
        hasher.update(name.as_bytes());
        hasher.update(&[0]);
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
    }
    Ok(format!("{PREFIX}{}", hex(&hasher.finish())))
}

/// Whether the value is a `sha256:` digest, the form the services accept.
#[must_use]
pub fn is_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(PREFIX) else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Read a sidecar path for the digest, for callers that hold only names.
#[must_use]
pub fn sidecar(directory: &Path, name: &str) -> PathBuf {
    directory.join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::{Sha256, digest, hex, is_digest, sidecar};

    static SEQUENCE: AtomicU32 = AtomicU32::new(0);

    fn hash(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex(&hasher.finish())
    }

    #[test]
    fn the_nist_vectors_match() {
        assert_eq!(
            hash(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hash(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hash(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    fn sidecars(label: &str, bodies: [&[u8]; 6]) -> (PathBuf, Vec<PathBuf>) {
        let sequence = SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "qr-package-{}-{label}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("root");
        let names = [
            "qr-gateway",
            "qr-data",
            "qr-model",
            "qr-backtest",
            "qr-evidence",
            "qr-simulation",
        ];
        let programs = names
            .iter()
            .zip(bodies)
            .map(|(name, body)| {
                let path = sidecar(&root, name);
                fs::write(&path, body).expect("sidecar");
                path
            })
            .collect();
        (root, programs)
    }

    #[test]
    fn the_same_sidecars_produce_the_same_digest() {
        let (root, programs) = sidecars("same", [b"a", b"b", b"c", b"d", b"e", b"f"]);
        let first = digest(&programs).expect("first");
        let second = digest(&programs).expect("second");
        assert_eq!(first, second);
        assert!(is_digest(&first), "{first}");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_changed_sidecar_changes_the_digest() {
        let (root, programs) = sidecars("one", [b"a", b"b", b"c", b"d", b"e", b"f"]);
        let before = digest(&programs).expect("before");
        fs::write(&programs[2], b"changed").expect("rewrite");
        let after = digest(&programs).expect("after");
        assert_ne!(before, after);
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn the_file_order_does_not_change_the_digest() {
        let (root, mut programs) = sidecars("order", [b"a", b"b", b"c", b"d", b"e", b"f"]);
        let ordered = digest(&programs).expect("ordered");
        programs.reverse();
        assert_eq!(ordered, digest(&programs).expect("reversed"));
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_missing_sidecar_is_reported() {
        let (root, programs) = sidecars("missing", [b"a", b"b", b"c", b"d", b"e", b"f"]);
        fs::remove_file(&programs[0]).expect("remove");
        let error = digest(&programs).expect_err("missing sidecar");
        assert!(error.contains("cannot read"), "{error}");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_digest_must_be_sha256_hex() {
        assert!(is_digest(&format!("sha256:{}", "a".repeat(64))));
        assert!(!is_digest(&"a".repeat(64)));
        assert!(!is_digest(&format!("sha256:{}", "A".repeat(64))));
        assert!(!is_digest("sha256:abc"));
    }
}
