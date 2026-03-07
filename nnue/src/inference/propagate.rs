#[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
pub use fallback::*;

#[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
pub use simdvec::*;

#[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
mod fallback {
    use utils::memory::Align64;

    use crate::{arch::*, inference::accumulator::HalfAcc, simd::simd};

    const L1_OFF: usize = L1 / 2;

    #[allow(clippy::needless_range_loop)]
    pub fn propagate_all_layers(nn: &NNUEData, stm: &HalfAcc, opp: &HalfAcc, obkt: usize) -> f32 {
        let mut ft_out = Align64([0; L1]);
        let mut l1_out = Align64([0.0; L2]);
        let mut l2_out = Align64([0.0; L3]);

        // ------------- Feature Transform --------------

        // SF style pairwise multiplied CReLU.
        let mut activate = |acc: &HalfAcc, offset: usize| {
            for i in 0..L1_OFF {
                let x = (acc[i] as i32).clamp(0, FT_QUANT);
                let y = (acc[i + L1_OFF] as i32).clamp(0, FT_QUANT);
                ft_out[offset + i] = ((x * y) >> (16 - L1Q_BITS)) as u8;
            }
        };

        activate(stm, 0);
        activate(opp, L1_OFF);

        // --------------------- L1 ---------------------

        let mut vals = [0; L2];

        // Affine transform.
        for i in 0..L1 {
            for j in 0..L2 {
                vals[j] += nn.l1w[obkt][j * L1 + i] as i32 * ft_out[i] as i32;
            }
        }

        // Dequantize and SCReLU.
        for i in 0..L2 {
            let v = vals[i] as f32 * L1_DEQUANT + nn.l1b[obkt][i];
            let clip = v.clamp(0.0, 1.0);
            l1_out[i] = clip * clip;
        }

        // --------------------- L2 ---------------------

        let mut vals = nn.l2b[obkt];

        // Affine transform.
        for i in 0..L2 {
            for j in 0..L3 {
                vals[j] += l1_out[i] * nn.l2w[obkt][j + i * L3];
            }
        }

        // SCReLU.
        for i in 0..L3 {
            let clip = vals[i].clamp(0.0, 1.0);
            l2_out[i] = clip * clip;
        }

        // --------------------- L3 ---------------------

        let mut l3_prods = [0.0; L3];
        for i in 0..L3 {
            l3_prods[i] = l2_out[i] * nn.l3w[obkt][i];
        }

        // Ensure same order of operations
        simd::reduce_add(&mut l3_prods, L3) + nn.l3b[obkt]
    }
}

#[cfg(any(target_feature = "avx2", target_feature = "avx512f"))]
mod simdvec {
    use utils::memory::Align64;

    use crate::{
        arch::{FT_QUANT, L1, L1_DEQUANT, L1Q_BITS, L2, L3, NNUEData},
        inference::{accumulator::HalfAcc, sparse::SparseMat},
        simd::simd::{self, CHUNK_SIZE_F32, cvt_i32_f32},
    };

    const L1_OFF: usize = L1 / 2;

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
        let mut ft_out = Align64([0u8; L1]);
        let mut l1_out = Align64([0.0; L2]);
        let mut l2_out = Align64([0.0; L3]);

        let mut sparse = SparseMat::default();

        let zero_i = simd::zeroed_i();
        let zero_f = simd::zeroed_f();
        let one_f = simd::from_val_f32(1.0);
        let ftqa = simd::from_val_i16(FT_QUANT as i16);
        let dequant = simd::from_val_f32(L1_DEQUANT);

        // ------------- Feature Transform --------------

        unsafe {
            // SF style pairwise multiplied CReLU.
            let mut activate = |acc: &HalfAcc, offset: usize| {
                let aptr = acc.as_ptr();
                let optr = ft_out.as_mut_ptr();

                for i in (0..L1_OFF).step_by(simd::CHUNK_SIZE_I16 * 4) {
                    let x0 = simd::from_ptr_i16(aptr.add(i + simd::CHUNK_SIZE_I16 * 0));
                    let x1 = simd::from_ptr_i16(aptr.add(i + simd::CHUNK_SIZE_I16 * 1));
                    let x2 = simd::from_ptr_i16(aptr.add(i + simd::CHUNK_SIZE_I16 * 2));
                    let x3 = simd::from_ptr_i16(aptr.add(i + simd::CHUNK_SIZE_I16 * 3));

                    let y0 = simd::from_ptr_i16(aptr.add(i + simd::CHUNK_SIZE_I16 * 0 + L1_OFF));
                    let y1 = simd::from_ptr_i16(aptr.add(i + simd::CHUNK_SIZE_I16 * 1 + L1_OFF));
                    let y2 = simd::from_ptr_i16(aptr.add(i + simd::CHUNK_SIZE_I16 * 2 + L1_OFF));
                    let y3 = simd::from_ptr_i16(aptr.add(i + simd::CHUNK_SIZE_I16 * 3 + L1_OFF));

                    let x0_clip = simd::shl_i16::<L1Q_BITS>(simd::clamp_i16(x0, zero_i, ftqa));
                    let x1_clip = simd::shl_i16::<L1Q_BITS>(simd::clamp_i16(x1, zero_i, ftqa));
                    let x2_clip = simd::shl_i16::<L1Q_BITS>(simd::clamp_i16(x2, zero_i, ftqa));
                    let x3_clip = simd::shl_i16::<L1Q_BITS>(simd::clamp_i16(x3, zero_i, ftqa));

                    let y0_clip = simd::min_i16(y0, ftqa);
                    let y1_clip = simd::min_i16(y1, ftqa);
                    let y2_clip = simd::min_i16(y2, ftqa);
                    let y3_clip = simd::min_i16(y3, ftqa);

                    let xy0 = simd::mulhi_i16(x0_clip, y0_clip);
                    let xy1 = simd::mulhi_i16(x1_clip, y1_clip);
                    let xy2 = simd::mulhi_i16(x2_clip, y2_clip);
                    let xy3 = simd::mulhi_i16(x3_clip, y3_clip);

                    let prod_u8_01 = simd::packus_i16_u8(xy0, xy1);
                    let prod_u8_23 = simd::packus_i16_u8(xy2, xy3);

                    simd::to_ptr_u8(optr.add(offset + i).cast(), prod_u8_01);
                    simd::to_ptr_u8(optr.add(offset + i + simd::CHUNK_SIZE_U8).cast(), prod_u8_23);

                    sparse.update(prod_u8_01, prod_u8_23);
                }
            };

            activate(stm, 0);
            activate(opp, L1_OFF);
        }

        // --------------------- L1 ---------------------

        let mut vals = Align64([0; L2]);

        let ft_out_32 = unsafe { &*ft_out.as_ptr().cast::<Align64<[i32; L1 / 4]>>() };

        let full_chunks = sparse.count - (sparse.count % 4);

        unsafe {
            let vptr = vals.as_mut_ptr();
            let wptr = nn.l1w[obkt].as_ptr();
            let bptr = nn.l1b[obkt].as_ptr();
            let optr = l1_out.as_mut_ptr();

            // Affine transform (full chunks).
            for c in (0..full_chunks).step_by(4) {
                let idx_0 = sparse.index_for(c + 0);
                let idx_1 = sparse.index_for(c + 1);
                let idx_2 = sparse.index_for(c + 2);
                let idx_3 = sparse.index_for(c + 3);

                let ft0 = simd::from_val_i32(*ft_out_32.get_unchecked(idx_0));
                let ft1 = simd::from_val_i32(*ft_out_32.get_unchecked(idx_1));
                let ft2 = simd::from_val_i32(*ft_out_32.get_unchecked(idx_2));
                let ft3 = simd::from_val_i32(*ft_out_32.get_unchecked(idx_3));

                for i in 0..L2 / simd::CHUNK_SIZE_F32 {
                    let v = simd::from_ptr_i32(vptr.add(i * simd::CHUNK_SIZE_F32));

                    let w0 = simd::from_ptr_i8(wptr.add(idx_0 * L2 * 4 + i * simd::CHUNK_SIZE_U8));
                    let w1 = simd::from_ptr_i8(wptr.add(idx_1 * L2 * 4 + i * simd::CHUNK_SIZE_U8));
                    let w2 = simd::from_ptr_i8(wptr.add(idx_2 * L2 * 4 + i * simd::CHUNK_SIZE_U8));
                    let w3 = simd::from_ptr_i8(wptr.add(idx_3 * L2 * 4 + i * simd::CHUNK_SIZE_U8));

                    let v = simd::dpbusd_i32(v, ft0, w0);
                    let v = simd::dpbusd_i32(v, ft1, w1);
                    let v = simd::dpbusd_i32(v, ft2, w2);
                    let v = simd::dpbusd_i32(v, ft3, w3);

                    simd::to_ptr_i32(vptr.add(i * simd::CHUNK_SIZE_F32), v);
                }
            }

            // Affine Transform (tail).
            for c in full_chunks..sparse.count {
                let idx = sparse.index_for(c);
                let ft = simd::from_val_i32(*ft_out_32.get_unchecked(idx));
                for i in 0..L2 / simd::CHUNK_SIZE_F32 {
                    let v = simd::from_ptr_i32(vptr.add(i * simd::CHUNK_SIZE_F32));
                    let w = simd::from_ptr_i8(wptr.add(idx * L2 * 4 + i * simd::CHUNK_SIZE_U8));

                    let v = simd::dpbusd_i32(v, ft, w);
                    simd::to_ptr_i32(vptr.add(i * simd::CHUNK_SIZE_F32), v);
                }
            }

            // Dequantize and SCReLU.
            for i in (0..L2).step_by(CHUNK_SIZE_F32) {
                let val = simd::from_ptr_i32(vptr.add(i));
                let val_f = cvt_i32_f32(val);
                let bias = simd::from_ptr_f32(bptr.add(i));

                let p = simd::fmadd_f32(val_f, dequant, bias);
                let c = simd::clamp_f32(p, zero_f, one_f);
                let out = simd::mul_f32(c, c);
                simd::to_ptr_f32(optr.add(i), out);
            }
        }

        // --------------------- L2 ---------------------

        unsafe {
            let bptr = nn.l2b[obkt].clone().as_mut_ptr();
            let wptr = nn.l2w[obkt].as_ptr();
            let optr = l2_out.as_mut_ptr();

            // Affine transform.
            for i in 0..L2 {
                let input = simd::from_val_f32(l1_out[i]);

                for j in (0..L3).step_by(simd::CHUNK_SIZE_F32) {
                    let w = simd::from_ptr_f32(wptr.add(j + i * L3));
                    let b = simd::from_ptr_f32(bptr.add(j));
                    let r = simd::fmadd_f32(input, w, b);
                    simd::to_ptr_f32(bptr.add(j), r);
                }
            }

            let zero = simd::zeroed_f();
            let one = simd::from_val_f32(1.0);

            // SCReLU.
            for i in (0..L3).step_by(simd::CHUNK_SIZE_F32) {
                let x = simd::from_ptr_f32(bptr.add(i));
                let clip = simd::clamp_f32(x, zero, one);
                let sqr = simd::mul_f32(clip, clip);
                simd::to_ptr_f32(optr.add(i), sqr);
            }
        }

        // --------------------- L3 ---------------------

        unsafe {
            let mut sum = simd::zeroed_f();

            let iptr = l2_out.as_ptr();
            let wptr = nn.l3w[obkt].as_ptr();

            // Affine transform.
            for i in (0..L3).step_by(simd::CHUNK_SIZE_F32) {
                let x = simd::from_ptr_f32(iptr.add(i));
                let w = simd::from_ptr_f32(wptr.add(i));
                sum = simd::fmadd_f32(x, w, sum);
            }

            simd::reduce_add_f32(sum) + nn.l3b[obkt]
        }
    }
}
