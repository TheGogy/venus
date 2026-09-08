pub use simdvec::*;

mod simdvec {
    use utils::memory::Align64;

    #[cfg(feature = "nnz_logging")]
    use crate::inference::sparse::NNZ_TRACKER;
    use crate::{
        arch::{EFF_L2_LEN, FT_QUANT, HalfAcc, L1_DEQUANT, L1_LEN, L1Q_SHIFT, L2_LEN, L3_LEN, NNUEData, PAIRWISE_LEN},
        inference::sparse::{NnzState, SparseMat},
        simd,
    };

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
        let dequant = simd::splat_f32(L1_DEQUANT);

        // ------------- Feature Transform --------------

        unsafe {
            let mut activate = |psqt: &HalfAcc, thrt: &HalfAcc, offset: usize| {
                let (psqt, thrt) = (psqt.as_ptr(), thrt.as_ptr());
                let ft_out_ptr = ft_out.as_mut_ptr();

                for i in (0..PAIRWISE_LEN).step_by(simd::I16_LANES * 4) {
                    // Sum together the psqt and threat accumulators.
                    let x0 = simd::add_i16(
                        simd::load_i16(psqt.add(i + simd::I16_LANES * 0)),
                        simd::load_i16(thrt.add(i + simd::I16_LANES * 0)),
                    );
                    let x1 = simd::add_i16(
                        simd::load_i16(psqt.add(i + simd::I16_LANES * 1)),
                        simd::load_i16(thrt.add(i + simd::I16_LANES * 1)),
                    );
                    let x2 = simd::add_i16(
                        simd::load_i16(psqt.add(i + simd::I16_LANES * 2)),
                        simd::load_i16(thrt.add(i + simd::I16_LANES * 2)),
                    );
                    let x3 = simd::add_i16(
                        simd::load_i16(psqt.add(i + simd::I16_LANES * 3)),
                        simd::load_i16(thrt.add(i + simd::I16_LANES * 3)),
                    );
                    let y0 = simd::add_i16(
                        simd::load_i16(psqt.add(i + simd::I16_LANES * 0 + PAIRWISE_LEN)),
                        simd::load_i16(thrt.add(i + simd::I16_LANES * 0 + PAIRWISE_LEN)),
                    );
                    let y1 = simd::add_i16(
                        simd::load_i16(psqt.add(i + simd::I16_LANES * 1 + PAIRWISE_LEN)),
                        simd::load_i16(thrt.add(i + simd::I16_LANES * 1 + PAIRWISE_LEN)),
                    );
                    let y2 = simd::add_i16(
                        simd::load_i16(psqt.add(i + simd::I16_LANES * 2 + PAIRWISE_LEN)),
                        simd::load_i16(thrt.add(i + simd::I16_LANES * 2 + PAIRWISE_LEN)),
                    );
                    let y3 = simd::add_i16(
                        simd::load_i16(psqt.add(i + simd::I16_LANES * 3 + PAIRWISE_LEN)),
                        simd::load_i16(thrt.add(i + simd::I16_LANES * 3 + PAIRWISE_LEN)),
                    );

                    // Clip both inputs to [0..FT_QUANT], giving CReLU on each half.
                    let x0_clip = simd::clamp_i16(x0, zero_i, ft_quant);
                    let x1_clip = simd::clamp_i16(x1, zero_i, ft_quant);
                    let x2_clip = simd::clamp_i16(x2, zero_i, ft_quant);
                    let x3_clip = simd::clamp_i16(x3, zero_i, ft_quant);

                    let y0_clip = simd::clamp_i16(y0, zero_i, ft_quant);
                    let y1_clip = simd::clamp_i16(y1, zero_i, ft_quant);
                    let y2_clip = simd::clamp_i16(y2, zero_i, ft_quant);
                    let y3_clip = simd::clamp_i16(y3, zero_i, ft_quant);

                    // Take the whole product and shift it back down to 0..=PAIRWISE_MAX.
                    let xy0 = simd::mulshr_u16::<L1Q_SHIFT>(x0_clip, y0_clip);
                    let xy1 = simd::mulshr_u16::<L1Q_SHIFT>(x1_clip, y1_clip);
                    let xy2 = simd::mulshr_u16::<L1Q_SHIFT>(x2_clip, y2_clip);
                    let xy3 = simd::mulshr_u16::<L1Q_SHIFT>(x3_clip, y3_clip);

                    // Pack i16 -> u8.
                    let prod_u8_01 = simd::packus_i16_u8(xy0, xy1);
                    let prod_u8_23 = simd::packus_i16_u8(xy2, xy3);

                    // Store in ft_out.
                    simd::store_u8(ft_out_ptr.add(offset + i + 0 * simd::U8_LANES).cast(), prod_u8_01);
                    simd::store_u8(ft_out_ptr.add(offset + i + 1 * simd::U8_LANES).cast(), prod_u8_23);

                    // Update sparse activation tracker.
                    let pack_i32_01 = simd::cast_u8_i32(prod_u8_01);
                    let pack_i32_23 = simd::cast_u8_i32(prod_u8_23);

                    sparse.update(&mut nnz, pack_i32_01, pack_i32_23);
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

                for i in 0..L2_LEN / simd::F32_LANES {
                    let x = simd::load_i32(acts_ptr.add(i * simd::F32_LANES));
                    let x = simd::dotprod_i32(x, ft0, simd::load_i8(weight_ptr.add(idx_0 * L2_LEN * 4 + i * simd::U8_LANES)));
                    let x = simd::dotprod_i32(x, ft1, simd::load_i8(weight_ptr.add(idx_1 * L2_LEN * 4 + i * simd::U8_LANES)));
                    let x = simd::dotprod_i32(x, ft2, simd::load_i8(weight_ptr.add(idx_2 * L2_LEN * 4 + i * simd::U8_LANES)));
                    let x = simd::dotprod_i32(x, ft3, simd::load_i8(weight_ptr.add(idx_3 * L2_LEN * 4 + i * simd::U8_LANES)));

                    simd::store_i32(acts_ptr.add(i * simd::F32_LANES), x);
                }
            }

            // Affine Transform (tail).
            for c in full_chunks..nnz.count {
                let idx = sparse.index_for(c);
                let ft = simd::cast_i32_u8(simd::splat_i32(*ft_out_32.get_unchecked(idx)));
                for i in 0..L2_LEN / simd::F32_LANES {
                    let x = simd::load_i32(acts_ptr.add(i * simd::F32_LANES));
                    let wgt = simd::load_i8(weight_ptr.add(idx * L2_LEN * 4 + i * simd::U8_LANES));

                    let x = simd::dotprod_i32(x, ft, wgt);
                    simd::store_i32(acts_ptr.add(i * simd::F32_LANES), x);
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
            let mut sum = simd::splat_f32(0.0);

            let l1_out_ptr = l1_out.as_ptr();
            let l2_out_ptr = l2_out.as_ptr();
            let weight_ptr = nn.l3w[obkt].as_ptr();

            // Affine over the skip connection.
            for i in (0..EFF_L2_LEN).step_by(simd::F32_LANES) {
                let x = simd::load_f32(l1_out_ptr.add(i));
                let wgt = simd::load_f32(weight_ptr.add(i));

                sum = simd::fmadd_f32(x, wgt, sum);
            }

            // L2 output into L3.
            for i in (0..L3_LEN).step_by(simd::F32_LANES) {
                let y = simd::load_f32(l2_out_ptr.add(i));
                let wgt = simd::load_f32(weight_ptr.add(EFF_L2_LEN + i));

                sum = simd::fmadd_f32(y, wgt, sum);
            }

            simd::reduce_add_f32(sum) + nn.l3b[obkt]
        }
    }
}
