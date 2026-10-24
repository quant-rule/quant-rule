//! A scalar reading series aligned to the input candle series.
//!
//! Every scalar reading has one entry per input candle. `None` means the
//! reading is unavailable at that bar; it is never a zero used to stand in for
//! missing data. A measured zero is stored as `Some(0.0)`.

/// A per-bar series of scalar readings.
///
/// The series is created by the reading functions in this module's parent.
/// Its length always equals the number of candles that were passed in, so a
/// consumer can index it by the candle's position without a separate lookup.
///
/// # Examples
///
/// ```
/// use quantrule_model::readings::ReadingSeries;
///
/// let series = ReadingSeries::new(vec![None, Some(0.0), Some(2.5)]);
/// assert_eq!(series.len(), 3);
/// assert_eq!(series.get(0), None);
/// assert_eq!(series.get(1), Some(0.0));
/// assert_eq!(series.get(2), Some(2.5));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ReadingSeries {
    values: Vec<Option<f64>>,
}

impl ReadingSeries {
    /// Creates a series from its per-bar values.
    ///
    /// The caller is responsible for supplying exactly one value per input
    /// candle; the reading functions in this module do so.
    #[must_use]
    pub fn new(values: Vec<Option<f64>>) -> Self {
        Self { values }
    }

    /// The per-bar values, in bar order.
    #[must_use]
    pub fn values(&self) -> &[Option<f64>] {
        &self.values
    }

    /// The value at `bar`, or `None` when the bar is out of range or the
    /// reading is unavailable there.
    #[must_use]
    pub fn get(&self, bar: usize) -> Option<f64> {
        self.values.get(bar).copied().flatten()
    }

    /// The number of bars in the series.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the series has no bars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::ReadingSeries;

    #[test]
    fn get_distinguishes_unavailable_from_zero() {
        let series = ReadingSeries::new(vec![None, Some(0.0)]);

        assert_eq!(series.get(0), None);
        assert_eq!(series.get(1), Some(0.0));
        assert_eq!(series.get(2), None);
    }

    #[test]
    fn values_preserve_bar_order() {
        let series = ReadingSeries::new(vec![Some(1.0), None, Some(3.0)]);

        assert_eq!(series.values(), [Some(1.0), None, Some(3.0)]);
    }

    #[test]
    fn an_empty_series_is_empty() {
        let series = ReadingSeries::new(Vec::new());

        assert_eq!(series.len(), 0);
        assert!(series.is_empty());
        assert_eq!(series.get(0), None);
    }
}
