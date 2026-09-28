use chess::{
    tables::{
        leaping_piece::{king_atk, knight_atk, pawn_atk},
        sliding_piece::{bishop_atk, rook_atk},
    },
    types::{
        bitboard::Bitboard,
        board::Board,
        color::Color,
        eval::Eval,
        moves::{Move, MoveFlag},
        piece::{CPiece, Piece},
        square::Square,
    },
};

use crate::tunables::params::{piece_value, tunables::value_pawn};

/// Static Exchange evaluation (SEE).
/// This determines if we win after all captures are made on a given square.
pub fn see(b: &Board, m: Move, threshold: Eval) -> bool {
    let (src, dst) = (m.src(), m.dst());
    let flag = m.flag();

    if flag == MoveFlag::Castling {
        return true;
    }

    // Get our piece that will be captured.
    let victim = if flag.is_promo() { CPiece::make(b.stm, flag.get_promo()) } else { b.pc_at(src) };

    // Get the value of the piece that we will use to capture.
    let mut move_val = if flag.is_cap() {
        if flag == MoveFlag::EnPassant { value_pawn() } else { piece_value(b.pc_at(dst)) }
    } else {
        0
    };

    if flag.is_promo() {
        move_val += piece_value(victim) - value_pawn();
    }

    // Stop if opponent is winning.
    let mut balance = move_val - threshold.0;
    if balance < 0 {
        return false;
    }

    // If balance is in our favor, we can stop now.
    balance -= piece_value(victim);
    if balance >= 0 {
        return true;
    }

    // Setup sliders.
    let diag_sliders = b.all_diag();
    let orth_sliders = b.all_orth();

    // Setup pins.
    let mut diag_pinned = [Bitboard::EMPTY; Color::NUM];
    let mut orth_pinned = [Bitboard::EMPTY; Color::NUM];

    for c in Color::iter() {
        diag_pinned[c.idx()] = b.state.pin_diag[c.idx()];
        orth_pinned[c.idx()] = b.state.pin_orth[c.idx()];

        // If the pinmask is aligned with the square we are capturing on, then only the pieces
        // that can't move in that direction are pinned.
        if b.state.pin_diag[c.idx()].has(dst) {
            diag_pinned[c.idx()] &= b.pc_bb(c, Piece::Knight) | b.pc_bb(c, Piece::Rook);
        }
        if b.state.pin_orth[c.idx()].has(dst) {
            orth_pinned[c.idx()] &= b.pc_bb(c, Piece::Pawn) | b.pc_bb(c, Piece::Bishop);
        }
    }

    let mut occ = b.occ();
    occ.pop(src);
    occ.pop(dst);

    if flag == MoveFlag::EnPassant {
        occ.pop(b.state.epsq.forward(!b.stm));
    }

    let mut atk = attackers_to(b, dst, occ) & occ;
    let mut stm = !b.stm;

    loop {
        let mut own_atk = atk & b.c_bb(stm);

        // Prune pinned pieces.
        if !(occ & b.c_bb(!stm) & b.state.pin_diag[stm.idx()]).is_empty() {
            own_atk &= !diag_pinned[stm.idx()];
        }
        if !(occ & b.c_bb(!stm) & b.state.pin_orth[stm.idx()]).is_empty() {
            own_atk &= !orth_pinned[stm.idx()];
        }

        // Exit when we run out of attackers.
        if own_atk.is_empty() {
            break;
        }

        // Get the least valuable attacker.
        let (p, s) = get_lva(b, stm, own_atk);
        occ.pop(s);

        let pt = p.pt();
        if matches!(pt, Piece::Queen | Piece::Bishop | Piece::Pawn) {
            atk |= bishop_atk(dst, occ) & diag_sliders;
        }
        if matches!(pt, Piece::Queen | Piece::Rook) {
            atk |= rook_atk(dst, occ) & orth_sliders;
        }

        atk &= occ;

        stm = !stm;
        balance = -balance - 1 - piece_value(p);
        if balance >= 0 {
            // If our final recapturing piece is a king, and the opponent has another attacker,
            // then a positive balance should mean a loss.
            if pt == Piece::King && !(atk & b.c_bb(stm)).is_empty() {
                return b.stm == stm;
            }

            break;
        }
    }

    stm != b.stm
}

/// Returns a bitboard of all pieces that can attack the given square.
#[rustfmt::skip]
fn attackers_to(b: &Board, s: Square, occ: Bitboard) -> Bitboard {
      b.pc_bb(Color::White, Piece::Pawn) & pawn_atk(Color::Black, s)
    | b.pc_bb(Color::Black, Piece::Pawn) & pawn_atk(Color::White, s)
    | b.p_bb(Piece::Knight)              & knight_atk(s)
    | b.p_bb(Piece::King)                & king_atk(s)
    | b.all_diag()                       & bishop_atk(s, occ)
    | b.all_orth()                       & rook_atk(s, occ)
}

/// Gets the least valuable attacker to a position.
fn get_lva(b: &Board, c: Color, atk: Bitboard) -> (CPiece, Square) {
    let my_occ = b.c_bb(c);

    for p in Piece::iter() {
        let attackers_bb = atk & b.pc_bb(c, p) & my_occ;

        if !attackers_bb.is_empty() {
            return (CPiece::make(c, p), attackers_bb.lsb());
        }
    }

    // The attacking bitboard will always contain at least one piece.
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tunables::params::tunables::{value_knight, value_pawn, value_queen, value_rook};

    /// Positions to check the SEE against, as (fen, move, threshold, expected).
    /// Thresholds are written in terms of the piece values so that they track any retuning.
    fn see_tests() -> Vec<(&'static str, &'static str, i32, bool)> {
        let (p, n, r, q) = (value_pawn(), value_knight(), value_rook(), value_queen());

        #[rustfmt::skip]
        let tests = vec![
            ("2k5/8/8/4p3/8/8/2K1R3/8 w - - 0 1", "e2e5", 0, true),
            ("3k4/8/8/4p3/3P4/8/8/5K2 w - - 0 1", "d4e5", p, true),
            ("3k4/8/5p2/4p3/3P4/8/8/5K2 w - - 0 1", "d4e5", p, false),
            ("8/3k4/2n2b2/8/3P4/8/3KN3/8 b - - 0 1", "c6d4", p, true),
            ("8/3k4/2n2b2/8/3P4/8/3KN3/8 b - - 0 1", "c6d4", n, false),
            ("3kr3/8/4q3/8/4P3/5P2/8/3K4 b - - 0 1", "e6e4", 0, false),
            ("3kr3/8/4q3/8/4P3/5P2/8/3K4 b - - 0 1", "e6e4", -q, true),
            ("8/3k4/2n2b2/8/3P4/3K4/4N3/8 b - - 0 1", "c6d4", p, false),
            ("5k2/2P5/4b3/8/8/8/8/2R2K2 w - - 0 1", "c7c8q", 0, true),
            ("5k2/2P5/4b3/8/8/8/8/3R1K2 w - - 0 1", "c7c8q", 0, false),
            ("8/3k2b1/2n2b2/8/3P4/3K4/4N3/8 b - - 0 1", "c6d4", 0, true),
            ("3k4/8/2q5/2b5/2r5/8/2P5/2R1K3 b - - 0 1", "c4c2", 0, false),
            ("3k4/8/2q5/2b5/2r5/8/2P5/2R1K3 b - - 0 1", "c4c2", p - r, true),
            ("2k5/3n2b1/2nq4/4R3/5P2/3N1N2/8/5K2 b - - 0 1", "d6e5", 0, false),
            ("2k5/3n2b1/2nq4/4R3/5P2/3N1N2/8/5K2 b - - 0 1", "d6e5", r - q + p, true),
            ("5r1k/3b1q1p/1npb4/1p6/pPpP1N2/2P4B/2NBQ1P1/5R1K b - - 0 1", "d6f4", 0, false),
            ("5r1k/3b1q1p/1npb4/1p6/pPpP1N2/2P4B/2NBQ1P1/5R1K b - - 0 1", "d6f4", -p, true),
        ];

        tests
    }

    #[test]
    fn test_see() {
        for (fen, mov, threshold, result) in see_tests() {
            let b: Board = fen.parse().unwrap();
            let m = b.find_move(mov).unwrap();
            assert_eq!(see(&b, m, Eval(threshold)), result, "{fen} {mov} @ {threshold}");
        }
    }
}
