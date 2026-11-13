//! The built client, served from a configured directory.
//!
//! The gateway is the only surface a client sees, so it serves the SPA at the
//! same origin as the API. A path outside the configured root is refused; an
//! unknown page falls back to `index.html`, but an `/api/*` path never does.

use std::path::{Path, PathBuf};

use axum::http::{Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};

use crate::runtime::server::AppState;

/// Serve one static request, or the SPA entry point.
pub async fn serve(state: &AppState, uri: &Uri, method: &Method) -> Response {
    let Some(root) = state.static_dir.as_deref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let path = uri.path();
    if path.starts_with("/api/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    if method != Method::GET && method != Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }

    let relative = path.trim_start_matches('/');
    let file = match file_under(root, Path::new(relative)).await {
        Some(file) => file,
        None => match file_under(root, Path::new("index.html")).await {
            Some(index) => index,
            None => return StatusCode::NOT_FOUND.into_response(),
        },
    };
    let body = match tokio::fs::read(&file).await {
        Ok(body) => body,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let cache = if path.starts_with("/assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type(&file)),
            (header::CACHE_CONTROL, cache.to_owned()),
        ],
        body,
    )
        .into_response()
}

/// Resolve a path under the root, refusing a directory, a missing file, or a
/// path that escapes the root through `..` or a symlink.
async fn file_under(root: &Path, relative: &Path) -> Option<PathBuf> {
    let root = tokio::fs::canonicalize(root).await.ok()?;
    let candidate = root.join(relative);
    let resolved = tokio::fs::canonicalize(&candidate).await.ok()?;
    if !resolved.starts_with(&root) {
        return None;
    }
    let metadata = tokio::fs::metadata(&resolved).await.ok()?;
    metadata.is_file().then_some(resolved)
}

fn content_type(path: &Path) -> String {
    let value = match path.extension().and_then(|value| value.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") | Some("map") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    };
    value.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static")
    }

    #[tokio::test]
    async fn a_file_under_the_root_is_found() {
        let found = file_under(&root(), Path::new("assets/app.js"))
            .await
            .expect("the fixture file");
        assert!(found.ends_with("assets/app.js"));
    }

    #[tokio::test]
    async fn a_parent_path_is_refused() {
        assert_eq!(file_under(&root(), Path::new("../secret")).await, None);
    }

    #[tokio::test]
    async fn a_directory_is_not_a_file() {
        assert_eq!(file_under(&root(), Path::new("assets")).await, None);
    }

    #[tokio::test]
    async fn the_content_type_follows_the_extension() {
        assert_eq!(
            content_type(Path::new("index.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            content_type(Path::new("app.js")),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(
            content_type(Path::new("app.css")),
            "text/css; charset=utf-8"
        );
        assert_eq!(
            content_type(Path::new("blob.bin")),
            "application/octet-stream"
        );
    }
}
