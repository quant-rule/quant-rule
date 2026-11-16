//! Shared fixtures and fakes for the backtest worker tests.
//!
//! The worker tests build their objects in memory and canonicalize a small CSV
//! the way ingest does, so no database, object store, or broker is required.

#![allow(dead_code)]

/// The in-memory object store from `quantrule_runtime`.
// Each integration target compiles this module separately, and not every
// target exercises the object store.
#[allow(unused_imports)]
pub use quantrule_runtime::adapters::memory::MemoryObjects;

/// Canonicalize a date-and-price CSV for a test, the way ingest does.
pub fn canonical(csv: &str) -> Vec<u8> {
    use quantrule_data::{Candle, RawCandle};

    let mut out = Vec::new();
    let mut index = 0_u64;
    for line in csv.lines() {
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() < 5 || !fields[0].starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        let raw = RawCandle {
            date: fields[0],
            open: fields[1],
            high: fields[2],
            low: fields[3],
            close: fields[4],
            volume: fields.get(5).copied(),
        };
        let candle = Candle::parse(index, raw).expect("test candle");
        out.extend_from_slice(candle.canonical_line().as_bytes());
        out.push(b'\n');
        index += 1;
    }
    out
}
