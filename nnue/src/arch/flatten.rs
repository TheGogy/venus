use crate::accumulator::SideAccumulator;

pub fn flatten(acc: &SideAccumulator, weights: &SideAccumulator) -> i32 {
    #[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
    {
        fallback::flatten(acc, weights)
    }

    #[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
    {
        simdvec::flatten(acc, weights)
    }
}

#[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
mod fallback {
    use crate::{accumulator::SideAccumulator, arch::QA};

    /// Squared Clipped ReLU
    pub fn screlu(x: i16) -> i32 {
        (x.clamp(0, QA as i16) as i32).pow(2)
    }

    /// Flatten the accumulator using the given weights. (fallback: non-vectorized)
    pub fn flatten(acc: &SideAccumulator, weights: &SideAccumulator) -> i32 {
        let mut sum = 0;

        for (&x, &w) in acc.0.iter().zip(&weights.0) {
            sum += screlu(x) * w as i32;
        }

        sum
    }
}

#[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
mod simdvec {
    use crate::{
        accumulator::SideAccumulator,
        arch::{L1, QA},
        simd::{self, CHUNK_SIZE},
    };

    /// Flatten the accumulator using the given weights. (vectorized)
    pub fn flatten(acc: &SideAccumulator, weights: &SideAccumulator) -> i32 {
        use simd::vi16::*;

        let min = zeroed();
        let max = from_val(QA as i16);

        let mut out = zeroed();

        for i in 0..L1 / CHUNK_SIZE {
            let vptr = unsafe { acc.0.as_ptr().add(i * CHUNK_SIZE) };
            let wptr = unsafe { weights.0.as_ptr().add(i * CHUNK_SIZE) };

            // Load and clip v, load w.
            let v = clamp(from_ptr(vptr), min, max);
            let w = from_ptr(wptr);

            // v * v + v * w
            let s = madd(v, mul(v, w));

            // Add to output.
            out = add(out, s);
        }

        sum(out)
    }
}
