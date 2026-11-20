//! Digest verification rendering.

use std::io::{self, Write};

/// Render a digest record.
pub(crate) fn digest(writer: &mut dyn Write, run_id: &str, digest: &str) -> io::Result<()> {
    writeln!(writer, "run_id: {run_id}")?;
    writeln!(writer, "digest: {digest}")
}
