//! A small deterministic generator.
//!
//! Resampling needs randomness, and the result must be reproducible. PCG32 with
//! an explicit seed gives both, without a dependency.

/// A PCG32 generator.
#[derive(Debug, Clone)]
pub struct Pcg32 {
    state: u64,
    increment: u64,
}

impl Pcg32 {
    /// Seed the generator. The same seed gives the same sequence forever.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let mut generator = Self {
            state: 0,
            increment: (seed << 1) | 1,
        };
        generator.next_u32();
        generator.state = generator.state.wrapping_add(seed);
        generator.next_u32();
        generator
    }

    /// The next 32-bit value.
    pub fn next_u32(&mut self) -> u32 {
        let previous = self.state;
        self.state = previous
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.increment);
        let shifted = (((previous >> 18) ^ previous) >> 27) as u32;
        let rotation = (previous >> 59) as u32;
        shifted.rotate_right(rotation)
    }

    /// A value in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
    }

    /// An index in `[0, len)`.
    ///
    /// # Panics
    ///
    /// Panics when `len` is zero, which a caller must exclude.
    pub fn below(&mut self, len: usize) -> usize {
        (self.next_f64() * len as f64) as usize % len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_sequence() {
        let mut first = Pcg32::new(7);
        let mut second = Pcg32::new(7);
        for _ in 0..10 {
            assert_eq!(first.next_u32(), second.next_u32());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut first = Pcg32::new(1);
        let mut second = Pcg32::new(2);
        assert_ne!(first.next_u32(), second.next_u32());
    }

    #[test]
    fn values_are_in_range() {
        let mut generator = Pcg32::new(3);
        for _ in 0..1_000 {
            let value = generator.next_f64();
            assert!((0.0..1.0).contains(&value), "{value}");
            assert!(generator.below(7) < 7);
        }
    }
}
