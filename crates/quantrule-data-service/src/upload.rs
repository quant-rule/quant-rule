//! Bounded request-body spooling.
//!
//! A dataset upload is never collected into one buffer. Each chunk is written
//! to a private temporary file as it arrives, and the cap is enforced before a
//! write, so a producer that outlives the limit is refused without reading the
//! rest of the body into memory. The file is removed when the request ends.
//!
//! The canonical candle copy is still materialized by the ingest step because
//! the object-store contract takes a byte vector; that limitation is recorded
//! in the plan and is not a second body buffer.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use axum::body::Body;
use futures_util::StreamExt;
use tokio::io::{AsyncWrite, AsyncWriteExt};

static NEXT: AtomicU64 = AtomicU64::new(1);

/// Why a body could not be spooled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpoolError {
    /// The body exceeded the configured cap.
    TooLarge,
    /// Reading the body failed.
    Read(String),
    /// Writing the temporary file failed.
    Write(String),
}

impl std::fmt::Display for SpoolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => formatter.write_str("the upload exceeds the limit"),
            Self::Read(message) => write!(formatter, "the upload could not be read: {message}"),
            Self::Write(message) => write!(formatter, "the upload could not be stored: {message}"),
        }
    }
}

impl std::error::Error for SpoolError {}

/// A spooled upload file, removed when it leaves scope.
#[derive(Debug)]
pub struct Spool {
    path: PathBuf,
    bytes: u64,
}

impl Spool {
    /// The private file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// How many bytes were written.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}

impl Drop for Spool {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Spool one body into a private file under `directory`.
///
/// # Errors
///
/// Returns [`SpoolError::TooLarge`] when the body passes `limit`, and a read or
/// write error when the transport or the filesystem fails.
pub async fn spool(body: Body, limit: u64, directory: &Path) -> Result<Spool, SpoolError> {
    let path = directory.join(format!(
        "quantrule-upload-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let file = tokio::fs::File::create(&path)
        .await
        .map_err(|error| SpoolError::Write(error.to_string()))?;
    let mut file = tokio::io::BufWriter::new(file);
    let bytes = match spool_to(body, limit, &mut file).await {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = tokio::fs::remove_file(&path).await;
            return Err(error);
        }
    };
    file.flush()
        .await
        .map_err(|error| SpoolError::Write(error.to_string()))?;
    Ok(Spool { path, bytes })
}

/// Write a body's chunks to one writer without buffering the whole body.
///
/// The function is generic so a test can observe that the first chunk reaches
/// the writer before the producer finishes. The limit is checked before each
/// write; an over-limit chunk is not written.
pub async fn spool_to<W>(body: Body, limit: u64, writer: &mut W) -> Result<u64, SpoolError>
where
    W: AsyncWrite + Unpin,
{
    let mut stream = body.into_data_stream();
    let mut written = 0_u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| SpoolError::Read(error.to_string()))?;
        let next = written.saturating_add(chunk.len() as u64);
        if next > limit {
            return Err(SpoolError::TooLarge);
        }
        writer
            .write_all(&chunk)
            .await
            .map_err(|error| SpoolError::Write(error.to_string()))?;
        written = next;
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures_util::stream;
    use tokio::sync::Notify;

    use super::*;

    #[tokio::test]
    async fn the_first_chunk_is_written_before_the_producer_finishes() {
        let first = Arc::new(Notify::new());
        let writer_first = Arc::clone(&first);
        let producer_first = Arc::clone(&first);

        let chunks = stream::unfold(0_u8, move |state| {
            let producer_first = Arc::clone(&producer_first);
            async move {
                match state {
                    0 => Some((Ok::<_, std::io::Error>(b"first".to_vec()), 1)),
                    1 => {
                        // Do not yield the second chunk until the writer has
                        // written the first one. A body that is collected
                        // before the first write would wait forever here.
                        producer_first.notified().await;
                        Some((Ok(b"second".to_vec()), 2))
                    }
                    _ => None,
                }
            }
        });
        let body = Body::from_stream(chunks);

        let mut writer = Observer {
            bytes: Vec::new(),
            first: writer_first,
        };
        let bytes = spool_to(body, 64, &mut writer).await.expect("spool");
        assert_eq!(bytes, 11);
        assert_eq!(writer.bytes, b"firstsecond");
    }

    #[tokio::test]
    async fn a_body_over_the_cap_is_refused_before_the_rest_is_read() {
        let chunks = stream::iter(vec![
            Ok::<_, std::io::Error>(vec![0_u8; 8]),
            Ok(vec![0_u8; 8]),
        ]);
        let body = Body::from_stream(chunks);
        let mut sink = tokio::io::sink();
        assert_eq!(
            spool_to(body, 10, &mut sink).await,
            Err(SpoolError::TooLarge)
        );
    }

    struct Observer {
        bytes: Vec<u8>,
        first: Arc<Notify>,
    }

    impl AsyncWrite for Observer {
        fn poll_write(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            self.bytes.extend_from_slice(buf);
            self.first.notify_one();
            std::task::Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Ok(()))
        }
    }
}
