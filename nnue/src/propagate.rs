#[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
pub use fallback::*;

#[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
pub use simdvec::*;

#[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
mod fallback {
    use crate::{accumulator::HalfAcc, arch::QA};

    /// Squared Clipped ReLU.
    pub fn screlu(x: i16) -> i32 {
        (x.clamp(0, QA as i16) as i32).pow(2)
    }

    /// Flatten the accumulator and propagate through L1 (fallback: non-vectorized).
    pub fn acc_l1(acc: &HalfAcc, weights: &HalfAcc) -> i32 {
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
        accumulator::HalfAcc,
        arch::{L1, QA},
        simd::vi16::*,
    };

    /// Flatten the accumulator and propagate through L1 (vectorized).
    pub fn acc_l1(acc: &HalfAcc, weights: &HalfAcc) -> i32 {
        let min = zeroed();
        let max = from_val(QA as i16);

        let mut out = zeroed();

        let vptr = acc.0.as_ptr();
        let wptr = weights.0.as_ptr();

        unsafe {
            for i in (0..L1).step_by(CHUNK_SIZE_I16) {
                // Load and clip v, load w.
                let v = clamp16(from_ptr(vptr.add(i)), min, max);
                let w = from_ptr(wptr.add(i));

                // v * v + v * w
                let s = madd16(v, mul16(v, w));

                // Add to output.
                out = add32(out, s);
            }
        }

        sum32(out)
    }
}
