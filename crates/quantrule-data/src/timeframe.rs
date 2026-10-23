//! Timeframes supported by the data engine.

/// A candle timeframe.
///
/// Only one-minute candles exist in the first slice. Higher timeframes are
/// built from M1 by issue #50.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeframe {
    seconds: u32,
}

impl Timeframe {
    /// One minute.
    pub const M1: Self = Self { seconds: 60 };

    /// The timeframe length in seconds.
    #[must_use]
    pub const fn seconds(self) -> u32 {
        self.seconds
    }
}

impl Default for Timeframe {
    fn default() -> Self {
        Self::M1
    }
}

#[cfg(test)]
mod tests {
    use super::Timeframe;

    #[test]
    fn m1_is_sixty_seconds() {
        assert_eq!(Timeframe::M1.seconds(), 60);
    }

    #[test]
    fn the_default_is_m1() {
        assert_eq!(Timeframe::default(), Timeframe::M1);
    }
}
