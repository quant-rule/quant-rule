//! Re-export of the shared signal contract.
//!
//! RA-009 moved the request and response to [`quantrule_api`] so the shell and
//! the browser can name the public contract without linking a service crate.
//! The re-export keeps the internal module path, the schema tokens, and the wire
//! bytes unchanged for every service that already imports them here.

pub use quantrule_api::signal::{
    SIGNAL_REQUEST_SCHEMA, SIGNAL_RESPONSE_SCHEMA, SignalRequest, SignalResponse,
};
