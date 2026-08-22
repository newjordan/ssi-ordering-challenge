//! Deterministic PRNG (SplitMix64) and Fisher–Yates shuffle. No external
//! crates; fully reproducible across runs and machines.

#[derive(Clone, Copy, Debug)]
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        SplitMix64(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..bound` (bound > 0), via Lemire's multiply-shift (small
    /// bias is irrelevant here; determinism is what matters).
    #[inline]
    pub fn below(&mut self, bound: usize) -> usize {
        (((self.next_u64() >> 32) * (bound as u64)) >> 32) as usize
    }

    /// In-place Fisher–Yates shuffle.
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        let n = v.len();
        if n < 2 {
            return;
        }
        for i in (1..n).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

/// A seed derived from a (family, index, n, nnz) tuple — stable across machines.
pub fn derive_seed(family: u64, index: u64, n: usize, nnz: usize) -> u64 {
    let mut r = SplitMix64::new(
        family
            .wrapping_mul(0x1000_0000_01B3)
            .wrapping_add(index.wrapping_mul(0x100_0000_01B3))
            .wrapping_add((n as u64).wrapping_mul(0x2545_F491_4F6C_DD1D))
            .wrapping_add(nnz as u64),
    );
    r.next_u64()
}
