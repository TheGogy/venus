use crate::{
    tables::{atk_by_type, leaping_piece::pawn_atk},
    types::{
        board::Board,
        moves::{Move, MoveFlag},
        piece::{CPiece, Piece},
    },
};

impl Board {
    /// Whether the given move is legal in this position.
    /// This assumes that there was a move with this represetation at some point,
    /// and that that move was legal, but might not be legal now.
    pub fn is_legal(&self, m: Move) -> bool {
        let flag = m.flag();

        let (src, dst) = (m.src(), m.dst());
        let src_piece = self.pc_at(src);
        let src_pt = src_piece.pt();

        let occ = self.occ();
        let stm_occ = self.c_bb(self.stm);
        let opp_occ = self.c_bb(!self.stm);

        let pin_orth = self.state.pin_orth;
        let pin_diag = self.state.pin_diag;
        let attacked = self.state.attacked;
        let checkers = self.state.checkers;
        let checkmask = self.state.checkmask;

        // If the piece we're trying to move from isn't one of ours then move doesn't exist.
        if src_piece == CPiece::None || src_piece.color() != self.stm {
            return false;
        }

        // We can't capture our own piece.
        if stm_occ.contains(dst) {
            return false;
        }

        // Handle pawns.
        if src_pt == Piece::Pawn {
            let one_sq_fwd = src.forward(self.stm);

            // Handle single pushes.
            if one_sq_fwd != dst || occ.contains(dst) || flag != MoveFlag::Normal {
                return false;
            }

            // Handle double pushes.
            if one_sq_fwd.forward(self.stm) != dst || occ.contains(dst) || occ.contains(one_sq_fwd) || flag != MoveFlag::DoublePush {
                return false;
            }

            // Handle captures.
            if !(pawn_atk(self.stm, src) & opp_occ).contains(dst) || flag != MoveFlag::Capture {
                return false;
            }

        // Handle everything else.
        } else if !atk_by_type(src_pt, src, occ).contains(dst) {
            return false;
        }

        // If our piece starts in the pinmask, it must also end in that pinmask.
        if (pin_orth.contains(src) && !pin_orth.contains(dst)) || (pin_diag.contains(src) && !pin_diag.contains(dst)) {
            return false;
        }

        if self.in_check() {
            if src_pt == Piece::King {
                // King cannot move into check.
                if attacked.contains(dst) {
                    return false;
                }
            } else {
                // If we have 2 checking pieces, we need to move the King.
                if checkers.multiple() {
                    return false;
                }

                // We have to get between attacker and King.
                if !checkmask.contains(dst) {
                    return false;
                }
            }
        }

        true
    }
}
