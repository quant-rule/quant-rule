//! The column registry for the research dataset.
//!
//! Every column the export writes has one spec: its name, axis, type, range,
//! nullability, reason column, code registry, and a one-sentence description.
//! The header, the data dictionary, and the layer writers all read the same
//! order, so a column cannot be silently added to one and not the others.

/// One exported column.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Column {
    /// The column name.
    pub(crate) name: &'static str,
    /// The field's axis or record kind.
    pub(crate) axis: &'static str,
    /// The value type: `int`, `float`, or `list<int>`.
    pub(crate) value_type: &'static str,
    /// The documented range.
    pub(crate) range: &'static str,
    /// Whether the column can be null.
    pub(crate) nullable: bool,
    /// The column that carries the reason when this one is null.
    pub(crate) reason: &'static str,
    /// The code registry for this column, or an empty string.
    pub(crate) codes: &'static str,
    /// One sentence stating the quantity.
    pub(crate) description: &'static str,
}

// The registry has one const constructor with the spec's eight fields; a
// builder struct would add a line per field to each of the 161 registry rows
// without removing a real invariant.
#[allow(clippy::too_many_arguments)]
const fn c(
    name: &'static str,
    axis: &'static str,
    value_type: &'static str,
    range: &'static str,
    nullable: bool,
    reason: &'static str,
    codes: &'static str,
    description: &'static str,
) -> Column {
    Column {
        name,
        axis,
        value_type,
        range,
        nullable,
        reason,
        codes,
        description,
    }
}

mod aggregates;
mod layer1;
mod layer2;
mod raw;

pub(crate) use aggregates::AGGREGATES;
pub use aggregates::aggregate_names;
pub(crate) use layer1::LAYER1;
pub use layer1::layer1_names;
pub use layer2::layer2_names;
pub(crate) use layer2::{RESOLUTIONS, block_columns};
pub(crate) use raw::RAW;
