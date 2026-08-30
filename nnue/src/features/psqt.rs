use arrayvec::ArrayVec;
use chess::types::{
    bitboard::Bitboard,
    board::Board,
    color::Color,
    moves::{Move, MoveFlag},
    piece::Piece,
    rank_file::File,
    square::Square,
};
use utils::{cfor, max, min};

use crate::{
    arch::{HALF_BUCKET_MAP, HalfAcc, L1_LEN, NNUEData},
    features::Accumulator,
    simd,
};

/// Total input features.
pub const PSQT_FEATURES: usize = Color::NUM * Piece::NUM * Square::NUM;

/// We use 32 King positions defined by [`BUCKET_MAP`].
/// If the King is on any of the E-H files, we mirror all the features.
pub const NB_INPUT_BUCKETS: usize = max!(HALF_BUCKET_MAP) + 1;
pub const INPUT_KING_POSNS: usize = NB_INPUT_BUCKETS * 2;

/// Make a full input bucket map from the half input buckets.
pub const fn make_bucket_map(half_buckets: [usize; 32], nb_buckets: usize) -> [usize; 64] {
    let mut bucket_map = [0; 64];
    cfor!(let mut i = 0; i < 64; i += 1; {
        bucket_map[i] = half_buckets[(i / 8) * 4 + [0, 1, 2, 3, 3, 2, 1, 0][i % 8]];
        // Separate bucket for mirrored features.
        if i % 8 > 3 {
            bucket_map[i] += nb_buckets;
        }
    });
    bucket_map
}

/// Full input expert map.
pub const BUCKET_MAP: [usize; Square::NUM] = make_bucket_map(HALF_BUCKET_MAP, NB_INPUT_BUCKETS);

/// Get the current input bucket to use.
pub const fn input_bucket(ksq: Square, c: Color) -> usize {
    BUCKET_MAP[ksq.relative(c).idx()]
}

/// Index of the feature "piece `p` of colour `c` stands on `s`" as seen by `pov`, whose king is on
/// `ksq`.
#[allow(clippy::cast_possible_truncation)]
fn idx(pov: Color, mut ksq: Square, p: Piece, c: Color, mut s: Square) -> u32 {
    const PIECE_STRIDE: usize = Square::NUM;
    const OPPONENT_STRIDE: usize = Square::NUM * Piece::NUM;
    const BUCKET_STRIDE: usize = PSQT_FEATURES;

    // Kings on the E-H files see a mirrored board, which [`BUCKET_MAP`] gives its own buckets.
    if ksq.file() >= File::FE {
        ksq = ksq.fliph();
        s = s.fliph();
    }

    let bucket = input_bucket(ksq, pov);
    let opponent = c.idx() ^ pov.idx();
    (bucket * BUCKET_STRIDE + opponent * OPPONENT_STRIDE + p.idx() * PIECE_STRIDE + s.relative(pov).idx()) as u32
}

/// One feature index, as seen from each perspective.
type PovFeat = [u32; Color::NUM];

/// The features a move toggles.
#[derive(Clone, Copy, Debug, Default)]
enum Toggles {
    /// Quiet | Double | Promo.
    /// (+moved dst, -moved src)
    Add1Sub1(PovFeat, PovFeat),

    /// Capture | EnPassant | Capture promo.
    /// (+moved dst, -moved src, -captured src)
    Add1Sub2(PovFeat, PovFeat, PovFeat),

    /// Castling.
    /// (+king dst, +rook dst, -king src, -rook src)
    Add2Sub2(PovFeat, PovFeat, PovFeat, PovFeat),

    /// No move.
    #[default]
    None,
}

/// The change to the input features caused by a single move.
#[derive(Clone, Copy, Debug, Default)]
pub struct PsqtDelta {
    toggles: Toggles,

    /// The side whose king left its input bucket, if either.
    /// That side has to be refreshed fully from scratch.
    refresh: Option<Color>,
}

impl PsqtDelta {
    pub fn new(b: &Board, m: Move) -> Self {
        let pc = b.pc_at(m.src());
        let (mc, src, dst, flag) = (pc.color(), m.src(), m.dst(), m.flag());

        // Promotions land as the promoted piece, everything else lands as itself.
        let dst_pt = if flag.is_promo() { flag.get_promo() } else { pc.pt() };

        let ksqs = [b.ksq(Color::White), b.ksq(Color::Black)];
        let feat =
            |p: Piece, c: Color, s: Square| -> PovFeat { [idx(Color::White, ksqs[0], p, c, s), idx(Color::Black, ksqs[1], p, c, s)] };

        let add1 = feat(dst_pt, mc, dst);
        let sub1 = feat(pc.pt(), mc, src);

        let toggles = match flag {
            // En passant captures a piece that isn't on `dst` - have to handle separately.
            MoveFlag::EnPassant => Toggles::Add1Sub2(add1, sub1, feat(Piece::Pawn, !mc, dst.forward(!mc))),

            // Castling requires moving the rook features.
            MoveFlag::Castling => {
                let (rf, rt) = b.castlingmask.rook_src_dst(dst);
                Toggles::Add2Sub2(add1, feat(Piece::Rook, mc, rt), sub1, feat(Piece::Rook, mc, rf))
            }

            // All other captures / capture promos take the piece on `dst`.
            f if f.is_cap() => Toggles::Add1Sub2(add1, sub1, feat(b.pc_at(dst).pt(), !mc, dst)),

            // Normal / double / regular promo moves just add and subtract one piece.
            _ => Toggles::Add1Sub1(add1, sub1),
        };

        let king_changed = pc.pt() == Piece::King && input_bucket(src, mc) != input_bucket(dst, mc);
        Self { toggles, refresh: king_changed.then_some(mc) }
    }

    fn apply(&self, nn: &NNUEData, curr: &mut HalfAcc, prev: &HalfAcc, pov: Color) {
        debug_assert!(self.refresh != Some(pov), "applied a delta across a king bucket change");

        let row = |feat: u32| nn.ftw[feat as usize].as_ptr();
        let mut add_sub = |adds: &[*const i16], subs: &[*const i16]| unsafe {
            accumulate::<DELTA_REGS>(prev.as_ptr(), &[curr.as_mut_ptr()], adds, subs);
        };

        let p = pov.idx();
        match self.toggles {
            Toggles::Add1Sub1(a, s) => add_sub(&[row(a[p])], &[row(s[p])]),
            Toggles::Add1Sub2(a, s0, s1) => add_sub(&[row(a[p])], &[row(s0[p]), row(s1[p])]),
            Toggles::Add2Sub2(a0, a1, s0, s1) => add_sub(&[row(a0[p]), row(a1[p])], &[row(s0[p]), row(s1[p])]),
            Toggles::None => unreachable!("Applied a delta with no move behind it!!"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PsqtAccumulator {
    pub values: [HalfAcc; Color::NUM],
    delta: PsqtDelta,
    correct: [bool; Color::NUM],
}

impl Accumulator for PsqtAccumulator {
    type Cache = PsqtCache;

    fn new(nn: &NNUEData) -> Self {
        Self { values: [nn.ftb; Color::NUM], delta: PsqtDelta::default(), correct: [false; Color::NUM] }
    }

    fn push_move(&mut self, b: &Board, m: Move) {
        self.correct = [false; Color::NUM];
        self.delta = PsqtDelta::new(b, m);
    }

    fn correct(&self, pov: Color) -> bool {
        self.correct[pov.idx()]
    }

    fn requires_full_refresh(&self, pov: Color) -> bool {
        self.delta.refresh == Some(pov)
    }

    fn apply_delta(&mut self, nn: &NNUEData, prev: &Self, pov: Color) {
        self.delta.apply(nn, &mut self.values[pov.idx()], &prev.values[pov.idx()], pov);
        self.correct[pov.idx()] = true;
    }

    fn refresh(&mut self, nn: &NNUEData, cache: &mut PsqtCache, b: &Board, pov: Color) {
        let ksq = b.ksq(pov);
        let entry = &mut cache.0[input_bucket(ksq, pov)];

        let mut adds = ArrayVec::<*const i16, 32>::new();
        let mut subs = ArrayVec::<*const i16, 32>::new();

        for c in Color::iter() {
            for p in Piece::iter() {
                let old = entry.pieces[pov.idx()][p.idx()] & entry.colors[pov.idx()][c.idx()];
                let new = b.pc_bb(c, p);

                for sq in new & !old {
                    adds.push(nn.ftw[idx(pov, ksq, p, c, sq) as usize].as_ptr());
                }
                for sq in old & !new {
                    subs.push(nn.ftw[idx(pov, ksq, p, c, sq) as usize].as_ptr());
                }
            }
        }

        // Update the cache entry and the accumulator.
        let feats = entry.values[pov.idx()].as_mut_ptr();
        let dst = self.values[pov.idx()].as_mut_ptr();
        unsafe { accumulate::<REFRESH_REGS>(feats, &[dst, feats], &adds, &subs) }

        entry.pieces[pov.idx()] = b.pieces;
        entry.colors[pov.idx()] = b.colors;

        self.correct[pov.idx()] = true;
    }
}

#[derive(Clone, Debug)]
struct PsqtCacheEntry {
    values: [HalfAcc; Color::NUM],
    colors: [[Bitboard; Color::NUM]; Color::NUM],
    pieces: [[Bitboard; Piece::NUM]; Color::NUM],
}

impl PsqtCacheEntry {
    const fn new(nn: &NNUEData) -> Self {
        Self {
            values: [nn.ftb; Color::NUM],
            colors: [[Bitboard::EMPTY; Color::NUM]; Color::NUM],
            pieces: [[Bitboard::EMPTY; Piece::NUM]; Color::NUM],
        }
    }
}

#[derive(Clone, Debug)]
pub struct PsqtCache(Box<[PsqtCacheEntry; INPUT_KING_POSNS]>);

impl PsqtCache {
    /// # Panics
    /// Panics if the cache cannot be allocated.
    pub fn new(nn: &NNUEData) -> Self {
        let arr = (0..INPUT_KING_POSNS)
            .map(|_| PsqtCacheEntry::new(nn))
            .collect::<Vec<_>>()
            .into_boxed_slice()
            .try_into()
            .expect("Could not allocate NNUE weights!");
        Self(arr)
    }

    pub fn reset(&mut self, nn: &NNUEData) {
        for e in self.0.iter_mut() {
            *e = PsqtCacheEntry::new(nn);
        }
    }
}

const REFRESH_REGS: usize = 8;
const DELTA_REGS: usize = 1;

const _: () = assert!(L1_LEN.is_multiple_of(REFRESH_REGS * simd::I16_LANES));
const _: () = assert!(L1_LEN.is_multiple_of(DELTA_REGS * simd::I16_LANES));

#[inline]
unsafe fn accumulate<const REGS: usize>(src: *const i16, outs: &[*mut i16], adds: &[*const i16], subs: &[*const i16]) {
    debug_assert!(L1_LEN.is_multiple_of(REGS * simd::I16_LANES));

    let pairs = min!(adds.len(), subs.len());
    let mut regs = [simd::splat_i16(0); REGS];

    for base in (0..L1_LEN).step_by(REGS * simd::I16_LANES) {
        let off = |r: usize| base + r * simd::I16_LANES;

        unsafe {
            for (r, v) in regs.iter_mut().enumerate() {
                *v = simd::load_i16(src.add(off(r)));
            }

            for j in 0..pairs {
                let (a, s) = (adds[j], subs[j]);
                for (r, v) in regs.iter_mut().enumerate() {
                    let o = off(r);
                    *v = simd::add_i16(*v, simd::sub_i16(simd::load_i16(a.add(o)), simd::load_i16(s.add(o))));
                }
            }
            for &a in &adds[pairs..] {
                for (r, v) in regs.iter_mut().enumerate() {
                    *v = simd::add_i16(*v, simd::load_i16(a.add(off(r))));
                }
            }
            for &s in &subs[pairs..] {
                for (r, v) in regs.iter_mut().enumerate() {
                    *v = simd::sub_i16(*v, simd::load_i16(s.add(off(r))));
                }
            }

            for &out in outs {
                for (r, v) in regs.iter().enumerate() {
                    simd::store_i16(out.add(off(r)), *v);
                }
            }
        }
    }
}
