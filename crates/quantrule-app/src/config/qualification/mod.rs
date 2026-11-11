//! `qualification.json`: the human-owned acceptance criteria.
//!
//! The file names the evidence partitions, the sample floor, the cost model, the
//! multiple-testing declaration, and the criteria for the dimensions the
//! protocol evaluates. It is written before the run. The parser validates the
//! file before the evidence bundle is read; it never reads a result, a candle,
//! or an account value.
//!
//! The schema is `quantrule.qualification/v1`.

mod parse;
mod types;

pub use parse::load;
pub use types::{Correction, CostModel, Dimension, MultipleTesting, Operator};
pub use types::{Criteria, Criterion, EvidenceArtifact, Partition};
