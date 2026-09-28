//! Small seedable PRNG (SplitMix64). Mine placement needs neither
//! cryptographic strength nor a dependency; it does need reproducible seeds
//! for tests.

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn seeded(seed: u64) -> Self {
        Self(seed)
    }

    pub fn from_time() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        Self(nanos as u64 ^ (nanos >> 64) as u64)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..bound`. Rejection sampling keeps it unbiased.
    pub fn below(&mut self, bound: usize) -> usize {
        assert!(bound > 0, "empty range");
        let bound = bound as u64;
        let zone = u64::MAX - u64::MAX % bound;
        loop {
            let v = self.next_u64();
            if v < zone {
                return (v % bound) as usize;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::seeded(7);
        let mut b = Rng::seeded(7);
        for _ in 0..10 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn below_stays_in_range_and_hits_every_value() {
        let mut rng = Rng::seeded(1);
        let mut seen = [false; 5];
        for _ in 0..200 {
            seen[rng.below(5)] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }
}
