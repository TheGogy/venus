use arrayvec::ArrayVec;
use chess::{
    tables::{atk_by_type, atk_by_type_const},
    types::{
        bitboard::Bitboard,
        board::Board,
        color::Color,
        moves::Move,
        piece::{CPiece, Piece},
        rank_file::Rank,
        square::Square,
    },
};
use utils::{cfor, memory::Align64};

use crate::{
    arch::{HalfAcc, L1_LEN, NNUEData},
    features::{
        Accumulator, accumulate,
        byteboard::updates::on_move,
        orient::Orient,
        pawn::{MAX_PAWN_DELTAS_PER_MOVE, PAWN_FEATURES, PawnDeltas, collect_pawn_indices},
    },
    simd,
};

/// Total threat features.
pub const THRT_FEATURES: usize = 59808;

/// Max active threat features.
pub const MAX_ACTIVE_THREATS: usize = 128;

const _: () = assert!(PAWN_FEATURES + THRT_FEATURES <= u16::MAX as usize);

/// Max deltas per move, for each of the add and sub lists.
///
/// A move is up to 4 single square edits (castling), and each edit toggles at most:
/// 8  outgoing attacks (4 orth + 4 diag OR 8 knight)
/// 16 incoming attacks (4 orth + 4 diag AND 8 knight)
/// 8  discovered attacks (one per direction)
/// Also leave room for one whole vector store past the last delta: see [`byteboard::vbmi2::push_focus`].
pub const MAX_THREAT_DELTAS_PER_MOVE: usize = 96;

/// Total features each delta can touch.
pub const MAX_DELTA_FEATURES: usize = MAX_THREAT_DELTAS_PER_MOVE + MAX_PAWN_DELTAS_PER_MOVE;

#[allow(clippy::cast_possible_truncation)]
const fn piece_attacks(cpiece: CPiece, sq: usize) -> Bitboard {
    let square = Square::from_raw(sq as u8);

    // We cannot have pawns on the 1st / 8th ranks, mask these attacks out.
    if matches!((cpiece.pt(), square.rank()), (Piece::Pawn, Rank::R1 | Rank::R8)) {
        Bitboard::EMPTY
    } else {
        atk_by_type_const(cpiece.pt(), square, cpiece.color())
    }
}

/// Map of which pieces threaten other pieces.
///
/// Some are redundant: Pawns -> Bishop is implied by Bishop -> Pawn.
/// Always keep the feature from the piece of higher value.
/// If the threat is skipped, use `-1`.
///
/// All pieces can threaten pieces of the same type. When this happens, we take the piece on the
/// higher square.
#[rustfmt::skip]
const PIECE_TARGET_MAP: [[i32; Piece::NUM]; Piece::NUM] = [
//    P   N   B   R   Q   K
    [-1,  0, -1,  1, -1, -1], // P
    [ 0,  1,  2,  3,  4, -1], // N
    [ 0,  1,  2,  3, -1, -1], // B
    [ 0,  1,  2,  3, -1, -1], // R
    [ 0,  1,  2,  3,  4, -1], // Q
    [-1, -1, -1, -1, -1, -1], // K
];

/// Number of victim types for each attacker type.
const PIECE_TARGET_COUNT: [i32; Piece::NUM] = [4, 10, 8, 8, 10, 0];

/// Squares each piece pseudo-attacks from each square.
#[allow(clippy::cast_possible_truncation)]
const PSEUDO_ATK: [[Bitboard; Square::NUM]; CPiece::NUM] = {
    let mut atk = [[Bitboard::EMPTY; 64]; 12];

    cfor!(let mut cidx = 0; cidx < 12; cidx += 1; {
        let cpiece = CPiece::from_raw(cidx as u8);
        cfor!(let mut s = 0; s < 64; s += 1; {
            atk[cidx][s] = piece_attacks(cpiece, s);
        });
    });

    atk
};

/// Total number of attacks for each piece.
/// The first value is the total number of squares threatened by all the pseudo attacks of that piece,
/// the second value is the running total of threats from all pieces up to that piece.
#[allow(clippy::cast_possible_wrap)]
const PIECE_TOTAL_ATTACKS: [(i32, i32); CPiece::NUM] = {
    let mut tot_atk = [(0, 0); 12];

    let mut global_nb_attacks = 0;
    cfor!(let mut c = 0; c < 2; c += 1; {
        cfor!(let mut pt = 0; pt < 6; pt += 1; {
            let cpiece = CPiece::make(Color::from_raw(c), Piece::from_raw(pt));

            let mut pc_nb_attacks = 0;
            cfor!(let mut sq = 0; sq < 64; sq += 1; {
                pc_nb_attacks += piece_attacks(cpiece, sq).nbits() as i32;
            });

            tot_atk[cpiece.idx()] = (pc_nb_attacks, global_nb_attacks);
            global_nb_attacks += PIECE_TARGET_COUNT[pt as usize] * pc_nb_attacks;
        });
    });

    tot_atk
};

/// Offset for each source square.
/// This table holds the cumulative count of the number of attack squares, which can then be used as
/// a relative offset in the feature array.
#[allow(clippy::cast_possible_truncation)]
const ATTACKER_SQ_OFFSET: [[u32; Square::NUM]; CPiece::NUM] = {
    let mut attacker_sq_offset = [[0; 64]; 12];

    cfor!(let mut cidx = 0; cidx < 12; cidx += 1; {
        let cpiece = CPiece::from_raw(cidx as u8);

        let mut piece_off = 0;
        cfor!(let mut sq = 0; sq < 64; sq += 1; {
            attacker_sq_offset[cidx][sq] = piece_off;
            piece_off += piece_attacks(cpiece, sq).nbits();
        });

    });

    attacker_sq_offset
};

/// Base feature index for a given (attacker, victim, direction).
/// If the feature is excluded, use `u32::MAX`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
const ATTACK_INDEX: [[[u32; 2]; CPiece::NUM]; CPiece::NUM] = {
    let mut atk_idx = [[[0; 2]; 12]; 12];

    cfor!(let mut a = 0; a < 12; a += 1; {
        let attacker = CPiece::from_raw(a as u8);
        let (piece_offset, global_offset) = PIECE_TOTAL_ATTACKS[a];

        cfor!(let mut v = 0; v < 12; v += 1; {
            let victim = CPiece::from_raw(v as u8);

            let map = PIECE_TARGET_MAP[attacker.pt().idx()][victim.pt().idx()];

            // Fully excluded: this threat is already handled by another piece of higher value, so
            // we never use these threats.
            let fully_excluded = map == -1;

            // Semi excluded: This threat is shared between two pieces, so only consider the feature
            // from the backward direction.
            let opposite_colors = attacker.color().to_raw() != victim.color().to_raw();
            let semi_excluded = attacker.pt().to_raw() == victim.pt().to_raw() && (opposite_colors || attacker.pt().to_raw() != Piece::Pawn.to_raw());

            // Shift to second half for black.
            let color_base = victim.color() as i32 * (PIECE_TARGET_COUNT[attacker.pt().idx()] / 2);
            let feat_idx = global_offset + (color_base + map) * piece_offset;

            assert!(fully_excluded || feat_idx >= 0);

            // Backward index.
            atk_idx[a][v][0] = if fully_excluded { u32::MAX } else { feat_idx as u32 };
            // Forward index.
            atk_idx[a][v][1] = if fully_excluded || semi_excluded { u32::MAX } else { feat_idx as u32 };
        });
    });

    atk_idx
};

/// Get the index into the threat features, or None if excluded.
#[allow(clippy::cast_possible_truncation)]
pub fn threat_index(orient: Orient, attacker: CPiece, victim: CPiece, src: Square, dst: Square) -> Option<u16> {
    let (atk, vic) = (orient.piece(attacker), orient.piece(victim));
    let (src, dst) = (orient.sq(src), orient.sq(dst));

    // Two pieces of the same type threaten each other: only keep one.
    let direction = usize::from(src < dst);

    // Get the block for this attacker threatening this victim (or u32::MAX if excluded).
    let base = ATTACK_INDEX[atk as usize][vic as usize][direction];

    // Get the sub block for the attacker being on this square.
    let square_offset = ATTACKER_SQ_OFFSET[atk as usize][src as usize];

    // Get the offset for the square the victim is on.
    let victim_offset = (PSEUDO_ATK[atk as usize][src as usize] & Bitboard::below_mask(Square::from_raw(dst))).nbits();

    // Total threat index.
    let index = base.wrapping_add(square_offset).wrapping_add(victim_offset).wrapping_add(PAWN_FEATURES as u32);
    (base != u32::MAX).then_some(index as u16)
}

/// Collect all the active threat indices for the given perspective.
pub fn collect_threat_indices(b: &Board, orient: Orient, indices: &mut ArrayVec<u16, MAX_ACTIVE_THREATS>) {
    let occ = b.occ();

    for src in occ {
        let attacker = b.pc_at(src);
        let threats = atk_by_type(attacker, src, occ) & occ;

        for dst in threats {
            let victim = b.pc_at(dst);
            if let Some(idx) = threat_index(orient, attacker, victim, src, dst) {
                indices.push(idx);
            }
        }
    }
}

/// One threat toggled by a move.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ThreatDelta {
    atk: CPiece,
    src: Square,
    vic: CPiece,
    dst: Square,
}

const _: () = assert!(size_of::<ThreatDelta>() == 4);
const _: () = assert!(
    std::mem::offset_of!(ThreatDelta, atk) == 0
        && std::mem::offset_of!(ThreatDelta, src) == 1
        && std::mem::offset_of!(ThreatDelta, vic) == 2
        && std::mem::offset_of!(ThreatDelta, dst) == 3
);

impl ThreatDelta {
    pub const fn new(atk: CPiece, src: Square, vic: CPiece, dst: Square) -> Self {
        Self { atk, src, vic, dst }
    }

    pub fn index(self, orient: Orient) -> Option<u16> {
        threat_index(orient, self.atk, self.vic, self.src, self.dst)
    }
}

pub type ThreatDeltaList = ArrayVec<ThreatDelta, MAX_THREAT_DELTAS_PER_MOVE>;

#[derive(Clone, Default)]
pub struct ThreatDeltas {
    pub adds: ThreatDeltaList,
    pub subs: ThreatDeltaList,
}

impl ThreatDeltas {
    pub fn clear(&mut self) {
        self.adds.clear();
        self.subs.clear();
    }

    /// Collect every threat `m` toggles. `b` must be the board BEFORE the move.
    pub fn set(&mut self, b: &Board, m: Move) {
        self.clear();
        on_move(self, b, m);
    }
}

#[derive(Clone)]
pub struct ThreatAccumulator {
    values: [HalfAcc; Color::NUM],
    threat_deltas: ThreatDeltas,
    pawn_deltas: PawnDeltas,
    correct: [bool; Color::NUM],
    king_changed: [bool; Color::NUM],
}

impl Accumulator for ThreatAccumulator {
    type Cache = ();

    fn new(_: &NNUEData) -> Self {
        Self {
            values: [Align64([0; L1_LEN]); Color::NUM],
            threat_deltas: ThreatDeltas::default(),
            pawn_deltas: PawnDeltas::default(),
            correct: [false; Color::NUM],
            king_changed: [false; Color::NUM],
        }
    }

    fn values(&self, pov: Color) -> [&HalfAcc; Color::NUM] {
        match pov {
            Color::White => [&self.values[0], &self.values[1]],
            Color::Black => [&self.values[1], &self.values[0]],
        }
    }

    fn push_move_before(&mut self, b: &Board, m: Move) {
        let pc = b.pc_at(m.src());

        self.correct = [false; Color::NUM];
        self.king_changed = [false; Color::NUM];
        self.king_changed[pc.color().idx()] = pc.pt() == Piece::King && m.src().is_kingside() != m.dst().is_kingside();

        self.threat_deltas.set(b, m);
        self.pawn_deltas.before = b.both_p_bb(Piece::Pawn);
    }

    fn push_move_after(&mut self, b: &Board) {
        self.pawn_deltas.after = b.both_p_bb(Piece::Pawn);
    }

    fn correct(&self, pov: Color) -> bool {
        self.correct[pov.idx()]
    }

    fn requires_full_refresh(&self, pov: Color) -> bool {
        self.king_changed[pov.idx()]
    }

    fn apply_delta(&mut self, nn: &NNUEData, prev: &Self, pov: Color, ksq: Square) {
        debug_assert!(!self.king_changed[pov.idx()], "Applied a delta across a king mirror!");

        let mut add_idxs = ArrayVec::<u16, MAX_DELTA_FEATURES>::new();
        let mut sub_idxs = ArrayVec::<u16, MAX_DELTA_FEATURES>::new();

        let orient = Orient::new(ksq, pov);

        // Pawn features.
        self.pawn_deltas.collect_indices(orient, &mut add_idxs, &mut sub_idxs);

        // Threat features.
        collect_delta_indices(&self.threat_deltas.adds, orient, &mut add_idxs);
        collect_delta_indices(&self.threat_deltas.subs, orient, &mut sub_idxs);

        let mut adds = ArrayVec::<*const i8, MAX_DELTA_FEATURES>::new();
        let mut subs = ArrayVec::<*const i8, MAX_DELTA_FEATURES>::new();
        feat_rows(nn, &add_idxs, &mut adds);
        feat_rows(nn, &sub_idxs, &mut subs);

        let (prev, curr) = (prev.values[pov.idx()].as_ptr(), self.values[pov.idx()].as_mut_ptr());
        prefetch_rows(&adds, &subs);
        unsafe { accumulate::<i8, REGS, false>(prev, curr, &adds, &subs) };
        self.correct[pov.idx()] = true;
    }

    fn refresh(&mut self, nn: &NNUEData, (): &mut Self::Cache, b: &Board, pov: Color) {
        let orient = Orient::new(b.ksq(pov), pov);

        let mut add_idxs = ArrayVec::<u16, MAX_ACTIVE_THREATS>::new();
        collect_pawn_indices(b, orient, &mut add_idxs);
        collect_threat_indices(b, orient, &mut add_idxs);

        let mut adds = ArrayVec::<*const i8, MAX_ACTIVE_THREATS>::new();
        feat_rows(nn, &add_idxs, &mut adds);

        prefetch_rows(&adds, &[]);
        unsafe { accumulate::<i8, REGS, true>(std::ptr::null(), self.values[pov.idx()].as_mut_ptr(), &adds, &[]) };
        self.correct[pov.idx()] = true;
    }
}

/// Push the feature each delta toggles,
fn collect_delta_indices(deltas: &[ThreatDelta], orient: Orient, idxs: &mut ArrayVec<u16, MAX_DELTA_FEATURES>) {
    for d in deltas {
        if let Some(idx) = d.index(orient) {
            idxs.push(idx);
        }
    }
}

/// Resolve feature indices to the corresponding weight rows.
fn feat_rows<const N: usize>(nn: &NNUEData, feats: &[u16], rows: &mut ArrayVec<*const i8, N>) {
    debug_assert!(feats.len() <= rows.remaining_capacity());
    for &feat in feats {
        unsafe { rows.push_unchecked(nn.ftw_thrt[feat as usize].as_ptr()) };
    }
}

/// With 512 bit registers there are enough of them to hold the whole accumulator at once.
const REGS: usize = if simd::I16_LANES == 32 { L1_LEN / simd::I16_LANES } else { 8 };

const _: () = assert!(L1_LEN.is_multiple_of(REGS * simd::I16_LANES));

/// Pull every row about to be accumulated into cache.
fn prefetch_rows(adds: &[*const i8], subs: &[*const i8]) {
    for row in adds.iter().chain(subs) {
        simd::prefetch(row.cast());
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chess::types::{board::Board, color::Color, moves::Move, piece::Piece};

    use super::{
        ArrayVec, MAX_ACTIVE_THREATS, MAX_DELTA_FEATURES, Orient, PawnDeltas, ThreatDeltas, collect_pawn_indices, collect_threat_indices,
    };

    #[rustfmt::skip]
    const POSITIONS: &[&str] = &[
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1b1/2B1P1b1/P1NP1N1P/1PP1QPP1/R4RK1 w - - 0 10",

        "rbbknnqr/pppppppp/8/8/8/8/PPPPPPPP/RBBKNNQR w KQkq - 0 1",
        "bnrkrnqb/pppppppp/8/8/8/8/PPPPPPPP/BNRKRNQB w KQkq - 0 1",
        "1rqbkrbn/1ppppp1p/1n6/p1N3p1/8/2P4P/PP1PPPP1/1RQBKRBN w FBfb - 0 9",
    ];

    /// Multiset of every feature in the shared pawn pair and threat array for `pov`.
    fn active(b: &Board, pov: Color) -> HashMap<u16, i32> {
        let orient = Orient::new(b.ksq(pov), pov);
        let mut idxs = ArrayVec::<u16, MAX_ACTIVE_THREATS>::new();
        collect_pawn_indices(b, orient, &mut idxs);
        collect_threat_indices(b, orient, &mut idxs);

        let mut counts = HashMap::new();
        for i in idxs {
            *counts.entry(i).or_default() += 1;
        }
        counts
    }

    /// Applying the deltas of `m` to the features before it must give the features after it.
    fn check_move(b: &mut Board, m: Move) {
        let mut deltas = ThreatDeltas::default();
        deltas.set(b, m);

        let mut pawn_deltas = PawnDeltas { before: b.both_p_bb(Piece::Pawn), ..Default::default() };

        // A king crossing the mirror needs a full refresh, so no deltas.
        let pc = b.pc_at(m.src());
        let mirrored = pc.pt() == Piece::King && m.src().is_kingside() != m.dst().is_kingside();

        let before = [active(b, Color::White), active(b, Color::Black)];

        b.make_move(m);
        pawn_deltas.after = b.both_p_bb(Piece::Pawn);
        let after = [active(b, Color::White), active(b, Color::Black)];
        let ksqs = [b.ksq(Color::White), b.ksq(Color::Black)];
        b.undo_move();

        for pov in Color::iter() {
            if mirrored && pov == pc.color() {
                continue;
            }

            let orient = Orient::new(ksqs[pov.idx()], pov);

            let mut got = before[pov.idx()].clone();
            for (list, sign) in [(&deltas.adds, 1), (&deltas.subs, -1)] {
                for d in list {
                    if let Some(idx) = d.index(orient) {
                        *got.entry(idx).or_default() += sign;
                    }
                }
            }

            let mut pawn_adds = ArrayVec::<u16, MAX_DELTA_FEATURES>::new();
            let mut pawn_subs = ArrayVec::<u16, MAX_DELTA_FEATURES>::new();
            pawn_deltas.collect_indices(orient, &mut pawn_adds, &mut pawn_subs);
            for (list, sign) in [(&pawn_adds, 1), (&pawn_subs, -1)] {
                for &idx in list {
                    *got.entry(idx).or_default() += sign;
                }
            }

            got.retain(|_, &mut n| n != 0);

            assert!(
                got == after[pov.idx()],
                "{pov:?} features mismatch after {} in {}\n  missing: {:?}\n  extra:   {:?}",
                m.to_uci(&b.castlingmask),
                b.to_fen(),
                diff(&after[pov.idx()], &got),
                diff(&got, &after[pov.idx()]),
            );
        }
    }

    /// Features `a` has that `b` does not.
    fn diff(a: &HashMap<u16, i32>, b: &HashMap<u16, i32>) -> Vec<(u16, i32)> {
        let mut d: Vec<_> = a.iter().map(|(&i, &n)| (i, n - b.get(&i).copied().unwrap_or(0))).filter(|&(_, n)| n != 0).collect();
        d.sort_unstable();
        d
    }

    /// Check every move in the tree below `b`, down to `depth`.
    fn walk(b: &mut Board, depth: usize) {
        for m in b.gen_moves() {
            check_move(b, m);

            if depth > 1 {
                b.make_move(m);
                walk(b, depth - 1);
                b.undo_move();
            }
        }
    }

    const DEPTH: usize = 2;

    #[test]
    fn test_feature_deltas_match_refresh() {
        for fen in POSITIONS {
            let mut b: Board = fen.parse().unwrap();
            walk(&mut b, DEPTH);
        }
    }
}
