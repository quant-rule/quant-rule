//! The runtime-history receipt.
//!
//! Every owning service returns the same small shape after an erase: the record
//! identity and the bytes actually removed. A shared or still-referenced object
//! reports zero bytes, not the record's nominal size.

use serde::{Deserialize, Serialize};

/// The result of erasing one runtime record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EraseReceipt {
    /// The stable identity of the erased record.
    pub record_id: String,
    /// Bytes removed from the object store. Zero when the bytes are shared.
    pub bytes: u64,
}
