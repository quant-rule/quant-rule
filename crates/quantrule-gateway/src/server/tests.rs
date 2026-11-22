use axum::http::Method;
use quantrule_runtime::client::REQUEST_TIMEOUT;

use super::{LONG_REQUEST_TIMEOUT, proxy_deadline};

#[test]
fn long_routes_get_the_transfer_deadline() {
    assert_eq!(
        proxy_deadline(&Method::POST, "/catalog/datasets"),
        LONG_REQUEST_TIMEOUT
    );
    assert_eq!(
        proxy_deadline(&Method::POST, "/model/compute"),
        LONG_REQUEST_TIMEOUT
    );
    assert_eq!(
        proxy_deadline(&Method::GET, "/datasets/sha256:abc/candles"),
        LONG_REQUEST_TIMEOUT
    );
    assert_eq!(
        proxy_deadline(&Method::GET, "/runs/sha256:abc/artifacts/results.json"),
        LONG_REQUEST_TIMEOUT
    );
    assert_eq!(
        proxy_deadline(&Method::GET, "/stages/runs/abc/artifacts/results.json"),
        LONG_REQUEST_TIMEOUT
    );
    assert_eq!(
        proxy_deadline(&Method::GET, "/signals/sha256:abc/bytes"),
        LONG_REQUEST_TIMEOUT
    );
    assert_eq!(
        proxy_deadline(&Method::POST, "/signals"),
        LONG_REQUEST_TIMEOUT
    );
    assert_eq!(proxy_deadline(&Method::GET, "/signals"), REQUEST_TIMEOUT);
    assert_eq!(
        proxy_deadline(&Method::GET, "/signals/sha256:abc"),
        REQUEST_TIMEOUT
    );
    assert_eq!(proxy_deadline(&Method::GET, "/runs"), REQUEST_TIMEOUT);
    assert_eq!(proxy_deadline(&Method::POST, "/runs"), REQUEST_TIMEOUT);
    assert_eq!(
        proxy_deadline(&Method::GET, "/inspect/candles"),
        REQUEST_TIMEOUT
    );
}
