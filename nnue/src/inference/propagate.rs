use utils::memory::Align64;

#[cfg(feature = "nnz_logging")]
use crate::inference::sparse::NNZ_TRACKER;
use crate::{
    arch::{EFF_L2_LEN, FT_GATE_MAX, FT_QUANT, HalfAcc, L1_DEQUANT, L1_LEN, L1Q_SHIFT, L2_LEN, L3_LEN, NNUEData, PAIRWISE_LEN},
    inference::sparse::{NnzState, SparseMat},
    simd,
};

const L3_ACC_LANES: usize = 16;
const L3_ACC_REGS: usize = L3_ACC_LANES / simd::F32_LANES;

const _: () = assert!(L3_ACC_REGS.is_power_of_two());
const _: () = assert!(EFF_L2_LEN.is_multiple_of(L3_ACC_LANES) && L3_LEN.is_multiple_of(L3_ACC_LANES));

/// Fold the accumulator in half until one register is left and sum the values in it.
fn reduce_acc(mut acc: [simd::F32Vec; L3_ACC_REGS]) -> f32 {
    let mut regs = L3_ACC_REGS;
    while regs > 1 {
        regs /= 2;
        for r in 0..regs {
            acc[r] = simd::add_f32(acc[r], acc[r + regs]);
        }
    }
    simd::reduce_add_f32(acc[0])
}

/// `crelu(hardswish(y))` on the `FT_QUANT` grid, for `y` already clamped to `[0, FT_GATE_MAX]`:
/// `round(y * (y + 3 * FT_QUANT) / (6 * FT_QUANT))`.
const _: () = assert!(FT_QUANT == 255, "Gate constants are for FT_QUANT == 255. Update C1 and C2!!");

/// Constants for hardswish.
const C1: i16 = 76;
const C2: i16 = 27;

#[allow(clippy::cast_possible_truncation)]
const QA3: i16 = (FT_QUANT * 3) as i16;
#[allow(clippy::cast_possible_truncation)]
const QA6: i16 = (FT_QUANT * 6) as i16;

const S: i32 = (i16::MAX as i32) + 1;
const C: i32 = (C1 * C2) as i32;

#[allow(clippy::cast_possible_truncation)]
const CC: i16 = ((S * S) / (C * QA6 as i32)) as i16;

fn gate(y: simd::I16Vec) -> simd::I16Vec {
    let a = simd::mul_i16(y, simd::splat_i16(C1));
    let b = simd::mul_i16(simd::add_i16(y, simd::splat_i16(QA3)), simd::splat_i16(C2));
    simd::mulhrs_i16(simd::mulhrs_i16(a, b), simd::splat_i16(CC))
}

#[rustfmt::skip]
#[allow(
    clippy::erasing_op,
    clippy::identity_op,
    clippy::needless_range_loop,
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_ptr_alignment
)]
pub fn propagate_all_layers(nn: &NNUEData, psqt: [&HalfAcc; 2], thrt: [&HalfAcc; 2], obkt: usize) -> f32 {
    let mut ft_out = Align64([0u8; L1_LEN]);
    let mut l1_out = Align64([0.0; EFF_L2_LEN]);
    let mut l2_out = Align64([0.0; L3_LEN]);

    let mut sparse = SparseMat::default();
    let mut nnz = NnzState::default();

    let zero_i = simd::splat_i16(0);
    let zero_f = simd::splat_f32(0.0);
    let one_f = simd::splat_f32(1.0);
    let ft_quant = simd::splat_i16(FT_QUANT as i16);
    let gate_max = simd::splat_i16(FT_GATE_MAX as i16);
    let dequant = simd::splat_f32(L1_DEQUANT);

    // ------------- Feature Transform --------------

    unsafe {
        let mut activate = |psqt: &HalfAcc, thrt: &HalfAcc, offset: usize| {
            let (psqt, thrt) = (psqt.as_ptr(), thrt.as_ptr());
            let ft_out_ptr = ft_out.as_mut_ptr();

            for i in (0..PAIRWISE_LEN).step_by(simd::I16_LANES * 4) {
                // Sum the psqt and threat accumulators, then clip to [0..max].
                let half = |base: usize, max: simd::I16Vec| -> [simd::I16Vec; 4] {
                    std::array::from_fn(|r| {
                        let o = base + i + r * simd::I16_LANES;
                        simd::clamp_i16(simd::add_i16(simd::load_i16(psqt.add(o)), simd::load_i16(thrt.add(o))), zero_i, max)
                    })
                };
                let (x, y) = (half(0, ft_quant), half(PAIRWISE_LEN, gate_max));

                // Gate the second half, take the whole product, shift it back down to 0..=PAIRWISE_MAX, pack i16 -> u8.
                let xy: [simd::I16Vec; 4] = std::array::from_fn(|r| simd::mulshr_u16::<L1Q_SHIFT>(x[r], gate(y[r])));
                let prod = [simd::packus_i16_u8(xy[0], xy[1]), simd::packus_i16_u8(xy[2], xy[3])];

                simd::store_u8(ft_out_ptr.add(offset + i), prod[0]);
                simd::store_u8(ft_out_ptr.add(offset + i + simd::U8_LANES), prod[1]);

                // Update sparse activation tracker.
                sparse.update(&mut nnz, simd::cast_u8_i32(prod[0]), simd::cast_u8_i32(prod[1]));
            }
        };

        activate(psqt[0], thrt[0], 0);
        activate(psqt[1], thrt[1], PAIRWISE_LEN);
    }

    #[cfg(feature = "nnz_logging")]
    NNZ_TRACKER.with_borrow_mut(|t| t.update(&ft_out, nnz.count));

    // --------------------- L1 ---------------------

    let mut acts = Align64([0; L2_LEN]);

    let ft_out_32 = unsafe { &*ft_out.as_ptr().cast::<Align64<[i32; L1_LEN / 4]>>() };

    let full_chunks = nnz.count - (nnz.count % 4);

    unsafe {
        let acts_ptr = acts.as_mut_ptr();
        let weight_ptr = nn.l1w[obkt].as_ptr();
        let bias_ptr = nn.l1b[obkt].as_ptr();
        let l1_out_ptr = l1_out.as_mut_ptr();

        // Affine transform (full chunks).
        for c in (0..full_chunks).step_by(4) {
            let idx_0 = sparse.index_for(c + 0);
            let idx_1 = sparse.index_for(c + 1);
            let idx_2 = sparse.index_for(c + 2);
            let idx_3 = sparse.index_for(c + 3);

            let ft0 = simd::cast_i32_u8(simd::splat_i32(*ft_out_32.get_unchecked(idx_0)));
            let ft1 = simd::cast_i32_u8(simd::splat_i32(*ft_out_32.get_unchecked(idx_1)));
            let ft2 = simd::cast_i32_u8(simd::splat_i32(*ft_out_32.get_unchecked(idx_2)));
            let ft3 = simd::cast_i32_u8(simd::splat_i32(*ft_out_32.get_unchecked(idx_3)));

            for i in 0..L2_LEN / simd::I32_LANES {
                let x = simd::load_i32(acts_ptr.add(i * simd::I32_LANES));
                let x = simd::dotprod_i32(x, ft0, simd::load_i8(weight_ptr.add(idx_0 * L2_LEN * 4 + i * simd::U8_LANES)));
                let x = simd::dotprod_i32(x, ft1, simd::load_i8(weight_ptr.add(idx_1 * L2_LEN * 4 + i * simd::U8_LANES)));
                let x = simd::dotprod_i32(x, ft2, simd::load_i8(weight_ptr.add(idx_2 * L2_LEN * 4 + i * simd::U8_LANES)));
                let x = simd::dotprod_i32(x, ft3, simd::load_i8(weight_ptr.add(idx_3 * L2_LEN * 4 + i * simd::U8_LANES)));

                simd::store_i32(acts_ptr.add(i * simd::I32_LANES), x);
            }
        }

        // Affine Transform (tail).
        for c in full_chunks..nnz.count {
            let idx = sparse.index_for(c);
            let ft = simd::cast_i32_u8(simd::splat_i32(*ft_out_32.get_unchecked(idx)));
            for i in 0..L2_LEN / simd::I32_LANES {
                let x = simd::load_i32(acts_ptr.add(i * simd::I32_LANES));
                let wgt = simd::load_i8(weight_ptr.add(idx * L2_LEN * 4 + i * simd::U8_LANES));

                let x = simd::dotprod_i32(x, ft, wgt);
                simd::store_i32(acts_ptr.add(i * simd::I32_LANES), x);
            }
        }

        // Dequantize and activate.
        for i in (0..L2_LEN).step_by(simd::F32_LANES) {
            let val = simd::cvt_i32_f32(simd::load_i32(acts_ptr.add(i)));
            let bias = simd::load_f32(bias_ptr.add(i));

            let x = simd::fmadd_f32(val, dequant, bias);
            let x_sq = simd::mul_f32(x, x);

            simd::store_f32(l1_out_ptr.add(i), simd::clamp_f32(x, zero_f, one_f));
            simd::store_f32(l1_out_ptr.add(i + L2_LEN), simd::min_f32(x_sq, one_f));
        }
    }

    // --------------------- L2 ---------------------

    unsafe {
        let mut vals = nn.l2b[obkt];
        let act_ptr = vals.as_mut_ptr();

        let weight_ptr = nn.l2w[obkt].as_ptr();

        let l2_out_ptr = l2_out.as_mut_ptr();

        // Affine transform.
        for i in 0..EFF_L2_LEN {
            let input = simd::splat_f32(l1_out[i]);

            for j in (0..L3_LEN).step_by(simd::F32_LANES) {
                let wgt = simd::load_f32(weight_ptr.add(i * L3_LEN + j));
                let bias = simd::load_f32(act_ptr.add(j));
                let x = simd::fmadd_f32(input, wgt, bias);
                simd::store_f32(act_ptr.add(j), x);
            }
        }

        // SCReLU.
        for i in (0..L3_LEN).step_by(simd::F32_LANES) {
            let x = simd::load_f32(act_ptr.add(i));
            let clip = simd::clamp_f32(x, zero_f, one_f);

            let sqr = simd::mul_f32(clip, clip);
            simd::store_f32(l2_out_ptr.add(i), sqr);
        }
    }

    // --------------------- L3 ---------------------

    unsafe {
        let mut sum = [simd::splat_f32(0.0); L3_ACC_REGS];

        let l1_out_ptr = l1_out.as_ptr();
        let l2_out_ptr = l2_out.as_ptr();
        let weight_ptr = nn.l3w[obkt].as_ptr();

        // Affine over the skip connection.
        for i in (0..EFF_L2_LEN).step_by(L3_ACC_LANES) {
            for (r, acc) in sum.iter_mut().enumerate() {
                let o = i + r * simd::F32_LANES;
                let x = simd::load_f32(l1_out_ptr.add(o));
                let w = simd::load_f32(weight_ptr.add(o));
                *acc = simd::fmadd_f32(x, w, *acc);
            }
        }

        // L2 output into L3.
        for i in (0..L3_LEN).step_by(L3_ACC_LANES) {
            for (r, acc) in sum.iter_mut().enumerate() {
                let o = i + r * simd::F32_LANES;
                let x = simd::load_f32(l2_out_ptr.add(o));
                let w = simd::load_f32(weight_ptr.add(EFF_L2_LEN + o));
                *acc = simd::fmadd_f32(x, w, *acc);
            }
        }

        reduce_acc(sum) + nn.l3b[obkt]
    }
}

#[cfg(test)]
mod tests {
    use utils::memory::Align64;

    use super::gate;
    use crate::{
        arch::{FT_GATE_MAX, FT_QUANT},
        simd,
    };

    #[test]
    fn gate_matches_training() {
        for y in 0..=FT_GATE_MAX {
            let expected = ((2 * y * (y + 3 * FT_QUANT) + 6 * FT_QUANT) / (12 * FT_QUANT)).min(FT_QUANT);

            #[allow(clippy::cast_possible_truncation)]
            let g = gate(simd::splat_i16(y as i16));
            let mut out = Align64([0i16; simd::I16_LANES]);
            unsafe { simd::store_i16(out.as_mut_ptr(), g) };
            assert_eq!(i32::from(out[0]), expected, "y = {y}");
        }
    }
}
