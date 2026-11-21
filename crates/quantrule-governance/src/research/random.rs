//! CPython-compatible pseudo-random generation for the research harnesses.
//!
//! The evaluators recorded seeded controls with `random.Random`. Reproducing
//! those draws requires the same Mersenne Twister seeding, `_randbelow`,
//! `sample`, and `shuffle` rules; this module implements exactly that subset.

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7fff_ffff;

/// A CPython-compatible MT19937 generator.
pub struct Random {
    state: [u32; N],
    index: usize,
}

impl Random {
    /// Seed the generator the way `random.Random(seed)` does.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let mut state = [0_u32; N];
        let mut key = vec![(seed & 0xffff_ffff) as u32];
        if seed >> 32 != 0 {
            key.push((seed >> 32) as u32);
        }
        init_by_array(&mut state, &key);
        Self { state, index: N }
    }

    /// The next 32-bit output.
    pub fn next_u32(&mut self) -> u32 {
        if self.index >= N {
            for index in 0..N {
                let value =
                    (self.state[index] & UPPER_MASK) | (self.state[(index + 1) % N] & LOWER_MASK);
                self.state[index] = self.state[(index + M) % N]
                    ^ (value >> 1)
                    ^ if value & 1 == 1 { MATRIX_A } else { 0 };
            }
            self.index = 0;
        }
        let mut value = self.state[self.index];
        self.index += 1;
        value ^= value >> 11;
        value ^= (value << 7) & 0x9d2c_5680;
        value ^= (value << 15) & 0xefc6_0000;
        value ^= value >> 18;
        value
    }

    /// The next `random()` value in `[0, 1)`.
    pub fn random(&mut self) -> f64 {
        let first = self.next_u32();
        let second = self.next_u32();
        ((u64::from(first >> 5) * 67_108_864 + u64::from(second >> 6)) as f64)
            / 9_007_199_254_740_992.0
    }

    /// `getrandbits(bits)` for `bits <= 64`.
    pub fn getrandbits(&mut self, bits: u32) -> u64 {
        if bits == 0 {
            return 0;
        }
        if bits <= 32 {
            return u64::from(self.next_u32() >> (32 - bits));
        }
        let low = u64::from(self.next_u32());
        let high_bits = bits - 32;
        let high = u64::from(self.next_u32() >> (32 - high_bits));
        (high << 32) | low
    }

    /// `_randbelow(n)` for `n > 0`.
    pub fn randbelow(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        let bits = 64 - n.leading_zeros();
        loop {
            let value = self.getrandbits(bits);
            if value < n {
                return value;
            }
        }
    }

    /// `random.Random.sample(population, k)`.
    #[must_use]
    pub fn sample(&mut self, population: &[usize], k: usize) -> Vec<usize> {
        let n = population.len();
        let k = k.min(n);
        let mut result = Vec::with_capacity(k);
        let setsize = if k > 5 {
            21 + 4_usize.pow(ceil_log4((k * 3) as u64))
        } else {
            21
        };
        if n <= setsize {
            let mut pool = population.to_vec();
            for i in 0..k {
                let j = self.randbelow((n - i) as u64) as usize;
                result.push(pool[j]);
                pool[j] = pool[n - i - 1];
            }
        } else {
            let mut selected = std::collections::HashSet::new();
            for _ in 0..k {
                let mut j = self.randbelow(n as u64) as usize;
                while selected.contains(&j) {
                    j = self.randbelow(n as u64) as usize;
                }
                selected.insert(j);
                result.push(population[j]);
            }
        }
        result
    }

    /// `random.Random.shuffle(values)`.
    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let target = self.randbelow((index + 1) as u64) as usize;
            values.swap(index, target);
        }
    }
}

fn ceil_log4(value: u64) -> u32 {
    let mut power = 1_u64;
    let mut exponent = 0;
    while power < value {
        power *= 4;
        exponent += 1;
    }
    exponent
}

fn init_by_array(state: &mut [u32; N], key: &[u32]) {
    init_genrand(state, 19_650_218);
    let mut i = 1;
    let mut j = 0;
    for _ in 0..N.max(key.len()) {
        state[i] = (state[i] ^ ((state[i - 1] ^ (state[i - 1] >> 30)).wrapping_mul(1_664_525)))
            .wrapping_add(key[j])
            .wrapping_add(j as u32);
        i += 1;
        j += 1;
        if i >= N {
            state[0] = state[N - 1];
            i = 1;
        }
        if j >= key.len() {
            j = 0;
        }
    }
    for _ in 0..N - 1 {
        state[i] = (state[i] ^ ((state[i - 1] ^ (state[i - 1] >> 30)).wrapping_mul(1_566_083_941)))
            .wrapping_sub(i as u32);
        i += 1;
        if i >= N {
            state[0] = state[N - 1];
            i = 1;
        }
    }
    state[0] = 0x8000_0000;
}

fn init_genrand(state: &mut [u32; N], seed: u32) {
    state[0] = seed;
    for index in 1..N {
        state[index] = 1_812_433_253_u32
            .wrapping_mul(state[index - 1] ^ (state[index - 1] >> 30))
            .wrapping_add(index as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_zero_matches_cpython() {
        let mut random = Random::new(0);
        let first = random.random();
        let second = random.random();
        let third = random.random();
        assert!((first - 0.8444218515250481).abs() < 1e-15, "{first}");
        assert!((second - 0.7579544029403025).abs() < 1e-15, "{second}");
        assert!((third - 0.420571580830845).abs() < 1e-15, "{third}");
    }

    #[test]
    fn sample_matches_cpython() {
        let pool: Vec<usize> = (0..100).collect();
        let mut random = Random::new(0);
        let sampled = random.sample(&pool, 10);
        println!("sample0 {sampled:?}");
        assert_eq!(sampled, vec![49, 97, 53, 5, 33, 65, 62, 51, 38, 61]);
        let mut random = Random::new(1);
        assert_eq!(
            random.sample(&pool, 10),
            vec![17, 72, 97, 8, 32, 15, 63, 57, 60, 83]
        );
    }

    #[test]
    fn shuffle_matches_cpython() {
        let mut values: Vec<usize> = (0..20).collect();
        let mut random = Random::new(0);
        random.shuffle(&mut values);
        assert_eq!(
            values,
            vec![
                10, 18, 16, 14, 0, 17, 11, 2, 3, 9, 5, 7, 4, 19, 6, 15, 8, 1, 13, 12
            ]
        );
    }
}
