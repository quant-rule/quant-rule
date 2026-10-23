//! Shell-side market data sources.
//!
//! A source adapter owns only source access: it builds a request, performs the
//! transport call, parses the provider payload, and hands numeric values to the
//! data engine through `CandleWindow::push_values`. It does not validate
//! candles, assign series indices, or write output files. Those stay in the
//! engine and in the export module.
//!
//! The Dukascopy adapter is deliberately small. The provider endpoint is
//! undocumented, so the protocol details are isolated here and covered by
//! fixture tests rather than by the model or the data engine.

pub(crate) mod date;
pub(crate) mod dukascopy;
pub(crate) mod instrument;
pub(crate) mod jsonp;
pub(crate) mod transport;

pub(crate) use date::DateRange;
pub(crate) use dukascopy::{FetchError, FetchFailure, FetchRequest, RetryPolicy, fetch_candles};
pub(crate) use instrument::Symbol;
pub(crate) use transport::{HttpTransport, Transport};
