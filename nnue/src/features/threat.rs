/*
Threat inputs notes

For PSQT inputs, features are encoded as (piece, square, stm ksq) tuples.
For 16 input experts, this gives us 2 * 6 * 64 * 16 = 12288 features.

For threat inputs, features are encoded as (attacker, square, defender, square) tuples.
If we were to use all of that, it would be 2 * 6 * 64 * 2 * 6 * 64 = 589824 features.

Most of these features are redundant, we can reduce it by quite a bit:

1. Pieces can only attack certain squares, they will never threaten a square that is not in their pseudo-attack mask.
2. Features that imply other features are excluded (e.g Pawn -> Bishop is implied by Bishop -> Pawn).
3. If two pieces of the same type attack each other, one is redundant; Only keep if src > dst.
4. We don't call evaluation while in check: The King cannot be threatened.

Applying these restrictions, we get the following:

Pawn:   84   attacks * 6  victims = 504
Knight: 336  attacks * 10 victims = 3360
Bishop: 560  attacks * 8  victims = 4480
Rook:   896  attacks * 8  victims = 7168
Queen:  1456 attacks * 10 victims = 14560
King:   (ignored)

Total: 30072 * 2 sides = 60144 total inputs

*/

use chess::{
    tables::atk_by_type_const,
    types::{
        bitboard::Bitboard,
        color::Color,
        piece::{CPiece, Piece},
        rank_file::{File, Rank},
        square::Square,
    },
};
use utils::cfor;

const fn piece_attacks(cpiece: CPiece, sq: usize) -> usize {
    let square = Square::from_raw(sq as u8);

    // We cannot have pawns on the 1st / 8th ranks, mask these attacks out.
    if matches!((cpiece.pt(), square.rank()), (Piece::Pawn, Rank::R1 | Rank::R8)) {
        0
    } else {
        atk_by_type_const(cpiece.pt(), square, cpiece.color()).nbits() as usize
    }
}

/// Map of which pieces threaten other pieces.
///
/// Some are redundant: Pawns -> Bishop is implied by Bishop -> Pawn.
/// We always keep the feature from the piece of higher value.
/// If the threat is skipped, we use `-1`.
///
/// All pieces can threaten pieces of the same type. When this happens, we take the piece on the
/// higher square.
#[rustfmt::skip]
const PIECE_TARGET_MAP: [[i32; Piece::NUM]; Piece::NUM] = [
//    P   N   B   R   Q   K
    [ 0,  1, -1,  2, -1, -1], // P
    [ 0,  1,  2,  3,  4, -1], // N
    [ 0,  1,  2,  3, -1, -1], // B
    [ 0,  1,  2,  3, -1, -1], // R
    [ 0,  1,  2,  3,  4, -1], // Q
    [-1, -1, -1, -1, -1, -1], // K
];

/// Number of victim types for each attacker type.
const PIECE_TARGET_COUNT: [i32; Piece::NUM] = [6, 10, 8, 8, 10, 0];

/// For each (attacker, src, dst) combination, store the number of pseudo-attacked squares a victim
/// can be on.
/// Because all pieces can attack an opponent's piece of the same type (and we only want one feature)
/// we only count squares under the attacker square.
const VICTIM_SQ_OFFSET: [[[u8; Square::NUM]; Square::NUM]; CPiece::NUM] = {
    let mut atk_idx = [[[0u8; 64]; 64]; 12];

    cfor!(let mut cidx = 0; cidx < 12; cidx += 1; {
        let cpiece = CPiece::from_colored_idx(cidx);

        cfor!(let mut s = 0; s < 64; s += 1; {
            let src = Square::from_raw(s as u8);
            let atk = atk_by_type_const(cpiece.pt(), src, cpiece.color());

            cfor!(let mut d = 0; d < 64; d += 1; {
                let dst = Square::from_raw(d as u8);

                let atk_below = atk.0 & Bitboard::below_mask(dst).0;
                atk_idx[cidx][s][d] = atk_below.count_ones() as u8;
            });
        });
    });

    atk_idx
};

/// Total number of attacks for each piece.
/// The first value is the total number of squares threatened by all the pseudo attacks of that piece,
/// the second value is the running total of threats from all pieces up to that piece.
const PIECE_TOTAL_ATTACKS: [(i32, i32); CPiece::NUM] = {
    let mut tot_atk = [(0, 0); 12];

    let mut global_nb_attacks = 0;
    cfor!(let mut cidx = 0; cidx < 12; cidx += 1; {
        let cpiece = CPiece::from_colored_idx(cidx);

        let mut pc_nb_attacks = 0;
        cfor!(let mut sq = 0; sq < 64; sq += 1; {
            pc_nb_attacks += piece_attacks(cpiece, sq) as i32;
        });

        tot_atk[cidx] = (pc_nb_attacks, global_nb_attacks);
        global_nb_attacks += PIECE_TARGET_COUNT[cpiece.pt().idx()] * pc_nb_attacks;
    });

    tot_atk
};

/// Offset for each source square.
/// This table holds the cumulative count of the number of attack squares, which can then be used as
/// a relative offset in the feature array.
const ATTACKER_SQ_OFFSET: [[u32; Square::NUM]; CPiece::NUM] = {
    let mut attacker_sq_offset = [[0; 64]; 12];

    cfor!(let mut cidx = 0; cidx < 12; cidx += 1; {
        let cpiece = CPiece::from_colored_idx(cidx);

        let mut piece_off = 0;
        cfor!(let mut sq = 0; sq < 64; sq += 1; {
            attacker_sq_offset[cidx][sq] = piece_off;
            piece_off += piece_attacks(cpiece, sq) as u32;
        });

    });

    attacker_sq_offset
};

/// Base feature index for a given (attacker, victim, direction).
/// If the feature is excluded, use `u32::MAX`.
const ATTACK_INDEX: [[[u32; 2]; CPiece::NUM]; CPiece::NUM] = {
    let mut atk_idx = [[[0; 2]; 12]; 12];

    cfor!(let mut a = 0; a < 12; a += 1; {
        let attacker = CPiece::from_colored_idx(a);
        let (piece_offset, global_offset) = PIECE_TOTAL_ATTACKS[a];

        cfor!(let mut v = 0; v < 12; v += 1; {
            let victim = CPiece::from_colored_idx(v);

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

/// Get the index into the threat features.
/// Note that we should in theory use an `Option` here, but this way allows for branchless
/// execution. If we return `(false, ...)` then the index is excluded and should be discarded.
pub fn threat_index(ksq: Square, color: Color, mut attacker: CPiece, mut victim: CPiece, mut src: Square, mut dst: Square) -> (bool, u32) {
    // Threat indices are always from white's perspective.
    if color == Color::Black {
        attacker = attacker.flip_color();
        victim = victim.flip_color();
        src = src.flipv();
        dst = dst.flipv();
    }

    if ksq.file() >= File::FE {
        src = src.fliph();
        dst = dst.fliph();
    }

    // Two pieces of the same type threaten each other: only keep one.
    let direction = (src < dst) as usize;

    let attacker_idx = attacker.colored_idx();
    let victim_idx = victim.colored_idx();

    // Get the block for this attacker threatening this victim (or decide if it is excluded).
    let base = ATTACK_INDEX[attacker_idx][victim_idx][direction];

    // Get the sub block for the attacker being on this square.
    let square_offset = ATTACKER_SQ_OFFSET[attacker_idx][src.idx()];

    // Get the offset for the square the victim is on.
    let victim_offset = u32::from(VICTIM_SQ_OFFSET[attacker_idx][src.idx()][dst.idx()]);

    // Total threat index.
    let index = base.wrapping_add(square_offset).wrapping_add(victim_offset);

    (base != u32::MAX, index)
}
