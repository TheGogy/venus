#[cfg(not(any(target_feature = "avx2", target_feature = "avx512f", target_feature = "neon")))]
pub use fallback::*;
#[cfg(any(target_feature = "avx2", target_feature = "avx512f", target_feature = "neon"))]
pub use simdvec::*;

#[cfg(not(any(target_feature = "avx2", target_feature = "avx512f", target_feature = "neon")))]
mod fallback {
    use utils::memory::Align64;

    use crate::{
        arch::{EFF_L2_LEN, FT_QUANT, HalfAcc, L1_DEQUANT, L1_LEN, L1Q_BITS, L2_LEN, L3_LEN, NNUEData, PAIRWISE_LEN},
        simd::simd,
    };

    #[allow(clippy::needless_range_loop)]
    pub fn propagate_all_layers(nn: &NNUEData, stm: &HalfAcc, opp: &HalfAcc, obkt: usize) -> f32 {
        let mut ft_out = Align64([0; L1_LEN]);
        let mut l1_out = Align64([0.0; EFF_L2_LEN]);
        let mut l2_out = Align64([0.0; L3_LEN]);

        // ------------- Feature Transform --------------

        let mut activate = |acc: &HalfAcc, offset: usize| {
            for i in 0..PAIRWISE_LEN {
                let x = (acc[i] as i32).clamp(0, FT_QUANT);
                let y = (acc[i + PAIRWISE_LEN] as i32).clamp(0, FT_QUANT);
                ft_out[offset + i] = ((x * y) >> (16 - L1Q_BITS)) as u8;
            }
        };

        activate(stm, 0);
        activate(opp, PAIRWISE_LEN);

        // --------------------- L1 ---------------------

        let mut vals = [0; L2_LEN];

        // Affine transform.
        for i in 0..L1_LEN {
            for j in 0..L2_LEN {
                vals[j] += nn.l1w[obkt][i * L2_LEN + j] as i32 * ft_out[i] as i32;
            }
        }

        // Dequantize and activate.
        for i in 0..L2_LEN {
            let v = (vals[i] as f32).mul_add(L1_DEQUANT, nn.l1b[obkt][i]);

            l1_out[i] = v.clamp(0.0, 1.0);
            l1_out[i + L2_LEN] = (v * v).clamp(0.0, 1.0);
        }

        // --------------------- L2 ---------------------

        let mut vals = nn.l2b[obkt];

        // Affine transform.
        for i in 0..EFF_L2_LEN {
            for j in 0..L3_LEN {
                vals[j] = l1_out[i].mul_add(nn.l2w[obkt][i * L3_LEN + j], vals[j]);
            }
        }

        // SCReLU.
        for i in 0..L3_LEN {
            let clip = vals[i].clamp(0.0, 1.0);
            l2_out[i] = clip * clip;
        }

        // --------------------- L3 ---------------------

        let mut l3_prods = [0.0; L3_LEN];

        // Affine transform with skip conn.
        for i in 0..L3_LEN {
            l3_prods[i] = (l1_out[i] + l2_out[i]) * nn.l3w[obkt][i];
        }

        // Ensure same order of operations
        simd::reduce_add(&mut l3_prods, L3_LEN) + nn.l3b[obkt]
    }
}

#[cfg(any(target_feature = "avx2", target_feature = "avx512f", target_feature = "neon"))]
mod simdvec {
    use utils::memory::Align64;

    #[cfg(feature = "nnz_logging")]
    use crate::inference::sparse::NNZ_TRACKER;
    use crate::{
        arch::{EFF_L2_LEN, FT_QUANT, HalfAcc, L1_DEQUANT, L1_LEN, L1Q_BITS, L2_LEN, L3_LEN, NNUEData, PAIRWISE_LEN},
        inference::sparse::SparseMat,
        simd::simd,
    };

    #[allow(
        clippy::erasing_op,
        clippy::identity_op,
        clippy::needless_range_loop,
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::cast_ptr_alignment
    )]
    pub fn propagate_all_layers(nn: &NNUEData, stm: &HalfAcc, opp: &HalfAcc, obkt: usize) -> f32 {
        /// On ARM NEON, the mulhi instruction actually computes 2*x*y.
        /// This is effectively a left shift by itself, so when we shift left
        /// we do 1 less to compensate.
        const L1_DEQ_SHIFT: simd::ShiftT = L1Q_BITS - cfg!(target_feature = "neon") as simd::ShiftT;

        let mut ft_out = Align64([0u8; L1_LEN]);
        let mut l1_out = Align64([0.0; EFF_L2_LEN]);
        let mut l2_out = Align64([0.0; L3_LEN]);

        let mut sparse = SparseMat::default();

        let zero_i = simd::from_val_i16(0);
        let zero_f = simd::from_val_f32(0.0);
        let one_f = simd::from_val_f32(1.0);
        let ft_quant = simd::from_val_i16(FT_QUANT as i16);
        let dequant = simd::from_val_f32(L1_DEQUANT);

        // ------------- Feature Transform --------------

        unsafe {
            let mut activate = |accumulator: &HalfAcc, offset: usize| {
                let acc = accumulator.as_ptr();
                let ft_out_ptr = ft_out.as_mut_ptr();

                for i in (0..PAIRWISE_LEN).step_by(simd::CHUNK_SIZE_I16 * 4) {
                    let x0 = simd::from_ptr_i16(acc.add(i + simd::CHUNK_SIZE_I16 * 0));
                    let x1 = simd::from_ptr_i16(acc.add(i + simd::CHUNK_SIZE_I16 * 1));
                    let x2 = simd::from_ptr_i16(acc.add(i + simd::CHUNK_SIZE_I16 * 2));
                    let x3 = simd::from_ptr_i16(acc.add(i + simd::CHUNK_SIZE_I16 * 3));

                    let y0 = simd::from_ptr_i16(acc.add(i + simd::CHUNK_SIZE_I16 * 0 + PAIRWISE_LEN));
                    let y1 = simd::from_ptr_i16(acc.add(i + simd::CHUNK_SIZE_I16 * 1 + PAIRWISE_LEN));
                    let y2 = simd::from_ptr_i16(acc.add(i + simd::CHUNK_SIZE_I16 * 2 + PAIRWISE_LEN));
                    let y3 = simd::from_ptr_i16(acc.add(i + simd::CHUNK_SIZE_I16 * 3 + PAIRWISE_LEN));

                    // Clip y inputs from above.
                    // We don't care about clipping these from below:
                    // mulhi (positive << shift) * negative
                    // will be negative, so it will saturate to 0 with packus.
                    let y0_clip = simd::min_i16(y0, ft_quant);
                    let y1_clip = simd::min_i16(y1, ft_quant);
                    let y2_clip = simd::min_i16(y2, ft_quant);
                    let y3_clip = simd::min_i16(y3, ft_quant);

                    // Clip x inputs to [0..FT_QUANT] and then left shift so that
                    // mulhi gives CReLU(x) * CReLU(y)
                    let x0_clip = simd::shl_i16::<L1_DEQ_SHIFT>(simd::clamp_i16(x0, zero_i, ft_quant));
                    let x1_clip = simd::shl_i16::<L1_DEQ_SHIFT>(simd::clamp_i16(x1, zero_i, ft_quant));
                    let x2_clip = simd::shl_i16::<L1_DEQ_SHIFT>(simd::clamp_i16(x2, zero_i, ft_quant));
                    let x3_clip = simd::shl_i16::<L1_DEQ_SHIFT>(simd::clamp_i16(x3, zero_i, ft_quant));

                    let xy0 = simd::mulhi_i16(x0_clip, y0_clip);
                    let xy1 = simd::mulhi_i16(x1_clip, y1_clip);
                    let xy2 = simd::mulhi_i16(x2_clip, y2_clip);
                    let xy3 = simd::mulhi_i16(x3_clip, y3_clip);

                    // Pack i16 -> u8, saturate negatives to 0 (to clip y from below).
                    let prod_u8_01 = simd::packus_i16_u8(xy0, xy1);
                    let prod_u8_23 = simd::packus_i16_u8(xy2, xy3);

                    // Store in ft_out.
                    simd::to_ptr_u8(ft_out_ptr.add(offset + i + 0 * simd::CHUNK_SIZE_U8).cast(), prod_u8_01);
                    simd::to_ptr_u8(ft_out_ptr.add(offset + i + 1 * simd::CHUNK_SIZE_U8).cast(), prod_u8_23);

                    // Update sparse activation tracker.
                    let pack_i32_01 = simd::reinterpret_u8_i32(prod_u8_01);
                    let pack_i32_23 = simd::reinterpret_u8_i32(prod_u8_23);

                    sparse.update(pack_i32_01, pack_i32_23);
                }
            };

            activate(stm, 0);
            activate(opp, PAIRWISE_LEN);
        }

        #[cfg(feature = "nnz_logging")]
        NNZ_TRACKER.with_borrow_mut(|t| t.update(&ft_out, sparse.count));

        // --------------------- L1 ---------------------

        let mut acts = Align64([0; L2_LEN]);

        let ft_out_32 = unsafe { &*ft_out.as_ptr().cast::<Align64<[i32; L1_LEN / 4]>>() };

        let full_chunks = sparse.count - (sparse.count % 4);

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

                let ft0 = simd::reinterpret_i32_u8(simd::from_val_i32(*ft_out_32.get_unchecked(idx_0)));
                let ft1 = simd::reinterpret_i32_u8(simd::from_val_i32(*ft_out_32.get_unchecked(idx_1)));
                let ft2 = simd::reinterpret_i32_u8(simd::from_val_i32(*ft_out_32.get_unchecked(idx_2)));
                let ft3 = simd::reinterpret_i32_u8(simd::from_val_i32(*ft_out_32.get_unchecked(idx_3)));

                for i in 0..L2_LEN / simd::CHUNK_SIZE_F32 {
                    let x = simd::from_ptr_i32(acts_ptr.add(i * simd::CHUNK_SIZE_F32));

                    let x = simd::dotprod_i32(x, ft0, simd::from_ptr_i8(weight_ptr.add(idx_0 * L2_LEN * 4 + i * simd::CHUNK_SIZE_U8)));
                    let x = simd::dotprod_i32(x, ft1, simd::from_ptr_i8(weight_ptr.add(idx_1 * L2_LEN * 4 + i * simd::CHUNK_SIZE_U8)));
                    let x = simd::dotprod_i32(x, ft2, simd::from_ptr_i8(weight_ptr.add(idx_2 * L2_LEN * 4 + i * simd::CHUNK_SIZE_U8)));
                    let x = simd::dotprod_i32(x, ft3, simd::from_ptr_i8(weight_ptr.add(idx_3 * L2_LEN * 4 + i * simd::CHUNK_SIZE_U8)));

                    simd::to_ptr_i32(acts_ptr.add(i * simd::CHUNK_SIZE_F32), x);
                }
            }

            // Affine Transform (tail).
            for c in full_chunks..sparse.count {
                let idx = sparse.index_for(c);
                let ft = simd::reinterpret_i32_u8(simd::from_val_i32(*ft_out_32.get_unchecked(idx)));
                for i in 0..L2_LEN / simd::CHUNK_SIZE_F32 {
                    let x = simd::from_ptr_i32(acts_ptr.add(i * simd::CHUNK_SIZE_F32));
                    let wgt = simd::from_ptr_i8(weight_ptr.add(idx * L2_LEN * 4 + i * simd::CHUNK_SIZE_U8));

                    let x = simd::dotprod_i32(x, ft, wgt);
                    simd::to_ptr_i32(acts_ptr.add(i * simd::CHUNK_SIZE_F32), x);
                }
            }

            // Dequantize and activate.
            for i in (0..L2_LEN).step_by(simd::CHUNK_SIZE_F32) {
                let val = simd::cvt_i32_f32(simd::from_ptr_i32(acts_ptr.add(i)));
                let bias = simd::from_ptr_f32(bias_ptr.add(i));

                let x = simd::fmadd_f32(val, dequant, bias);
                let x_sq = simd::mul_f32(x, x);

                simd::to_ptr_f32(l1_out_ptr.add(i), simd::clamp_f32(x, zero_f, one_f));
                simd::to_ptr_f32(l1_out_ptr.add(i + L2_LEN), simd::min_f32(x_sq, one_f));
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
                let input = simd::from_val_f32(l1_out[i]);

                for j in (0..L3_LEN).step_by(simd::CHUNK_SIZE_F32) {
                    let wgt = simd::from_ptr_f32(weight_ptr.add(i * L3_LEN + j));
                    let bias = simd::from_ptr_f32(act_ptr.add(j));
                    let x = simd::fmadd_f32(input, wgt, bias);
                    simd::to_ptr_f32(act_ptr.add(j), x);
                }
            }

            // SCReLU.
            for i in (0..L3_LEN).step_by(simd::CHUNK_SIZE_F32) {
                let x = simd::from_ptr_f32(act_ptr.add(i));
                let clip = simd::clamp_f32(x, zero_f, one_f);

                let sqr = simd::mul_f32(clip, clip);
                simd::to_ptr_f32(l2_out_ptr.add(i), sqr);
            }
        }

        // --------------------- L3 ---------------------

        unsafe {
            let mut sum = simd::from_val_f32(0.0);

            let l1_out_ptr = l1_out.as_ptr();
            let l2_out_ptr = l2_out.as_ptr();
            let weight_ptr = nn.l3w[obkt].as_ptr();

            // Affine with skip conn.
            for i in (0..L3_LEN).step_by(simd::CHUNK_SIZE_F32) {
                let x = simd::from_ptr_f32(l1_out_ptr.add(i));
                let y = simd::from_ptr_f32(l2_out_ptr.add(i));
                let wgt = simd::from_ptr_f32(weight_ptr.add(i));

                sum = simd::fmadd_f32(simd::add_f32(x, y), wgt, sum);
            }

            simd::reduce_add_f32(sum) + nn.l3b[obkt]
        }
    }
}
