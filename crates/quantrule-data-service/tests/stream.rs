//! The upload is rejected while the producer is still sending.

mod common;

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{body_bytes, state_with_small_upload};
use futures_util::{StreamExt, stream};
use quantrule_api::PROFILE_HEADER;
use quantrule_data_service::router;
use tower::ServiceExt;

#[tokio::test]
async fn the_cap_is_applied_before_the_body_finishes() {
    // The first chunk already exceeds the 16-byte cap. The producer never
    // finishes, so a handler that collects the body would wait forever; the
    // bounded spool must answer 413 immediately.
    let first = stream::iter(vec![Ok::<_, std::io::Error>(vec![b'x'; 32])]);
    let body = Body::from_stream(first.chain(stream::pending()));
    let request = Request::post("/catalog/datasets?name=big.csv")
        .header(PROFILE_HEADER, "default")
        .body(body)
        .expect("request");

    let response = tokio::time::timeout(
        Duration::from_secs(2),
        router(state_with_small_upload().await).oneshot(request),
    )
    .await
    .expect("the handler waited for the rest of the body")
    .expect("response");
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("upload.too_large"), "{text}");
}

#[tokio::test]
async fn a_streamed_upload_without_a_content_length_is_accepted() {
    // Two chunks of canonical rows, no content-length, one stream. The route
    // must spool and ingest the body without a buffered extractor.
    let rows = concat!(
        "2024-01-01 00:00:00+00:00,100,101,99,100.5,10\n",
        "2024-01-01 00:01:00+00:00,100.5,101.5,100,101,12\n"
    );
    let middle = rows.len() / 2;
    let chunks = vec![
        Ok::<_, std::io::Error>(rows.as_bytes()[..middle].to_vec()),
        Ok(rows.as_bytes()[middle..].to_vec()),
    ];
    let request = Request::post("/catalog/datasets?name=stream.csv")
        .header(PROFILE_HEADER, "default")
        .body(Body::from_stream(stream::iter(chunks)))
        .expect("request");
    let response = router(common::state_with_catalog().await)
        .oneshot(request)
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let record = common::read_dataset(&body_bytes(response).await);
    assert_eq!(record.summary.candles, 2);
}
