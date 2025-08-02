use chess::{
    defs::MAX_PLY,
    types::{eval::Eval, moves::Move},
};

use crate::{
    movepick::{MovePicker, SearchType},
    position::Position,
    threading::thread::Thread,
    tt::{
        entry::{Bound, TT_DEPTH_OFFSET, TT_DEPTH_QS, TT_DEPTH_UNSEARCHED},
        table::TT,
    },
    tunables::params::tunables::*,
};

use super::NodeType;

impl Position {
    /// Quiescence search.
    /// We use this to avoid the "horizon" effect, by continuing the search
    /// until all captures have been made.
    pub fn qsearch<NT: NodeType>(&mut self, t: &mut Thread, tt: &TT, mut alpha: Eval, beta: Eval) -> Eval {
        // Check for upcoming repetition.
        if alpha < Eval::DRAW && self.board.upcoming_repetition(t.ply) {
            alpha = Eval::dithered_draw(t.nodes as i32);
            if alpha >= beta {
                return alpha;
            }
        }

        // Update seldepth.
        t.seldepth = t.seldepth.max(t.ply);

        let in_check = self.board.in_check();

        // Prevent infinite search.
        // If we're at maximum depth, we must return something reasonable.
        if t.ply >= MAX_PLY {
            return if in_check {
                Eval::DRAW
            } else {
                let raw = self.evaluate();
                self.adjust_eval(t, raw)
            };
        }

        // Stop searching if position is ruled as a draw.
        if self.board.is_draw(t.ply_from_null) {
            return Eval::DRAW;
        }

        // -----------------------------------
        //             TT lookup
        // -----------------------------------

        let mut tt_move = Move::NONE;
        let mut tt_value = -Eval::INFINITY;
        let mut tt_eval = -Eval::INFINITY;
        let mut tt_bound = Bound::None;
        let mut tt_depth = -TT_DEPTH_OFFSET;
        let mut tt_pv = false;

        if let Some(tte) = tt.probe(self.board.state.hash) {
            tt_move = tte.mov();
            tt_value = tte.value(t.ply);
            tt_eval = tte.eval();
            tt_bound = tte.bound();
            tt_depth = tte.depth();
            tt_pv = tte.pv();
        };

        // TT cutoff in qsearch.
        // If the bound from the tt is tighter than the current search value, just return it.
        // Even shallow TT entries can be useful in qsearch since we're mostly
        // looking at forced sequences.
        if !NT::PV && tt_value.is_valid() && tt_depth >= TT_DEPTH_QS && tt_bound.is_usable(tt_value, beta) {
            return tt_value;
        }

        // -----------------------------------
        //            Static Eval
        // -----------------------------------

        let mut best_value;
        let raw_value;
        let futility;

        if in_check {
            // When in check, we must search all evasions - can't stand pat
            best_value = -Eval::INFINITY;
            raw_value = -Eval::INFINITY;
            futility = -Eval::INFINITY;
        } else {
            // Stand pat evaluation: assume we can choose not to make any move.
            raw_value = if tt_eval.is_valid() { tt_eval } else { self.evaluate() };

            // Adjust evaluation with correction history.
            best_value = self.adjust_eval(t, raw_value);

            t.ss_mut().eval = best_value;

            // Futility pruning threshold for qsearch.
            // If our position + a reasonable bonus still can't reach alpha,
            // we can prune captures that don't improve the position significantly.
            futility = best_value + fp_qs_base();

            // Use TT score if it's more accurate than static eval.
            if tt_value.is_valid() && tt_bound.is_usable(tt_value, best_value) {
                best_value = tt_value
            }

            // Beta cutoff from stand pat.
            // If our current position is already good enough to cause a beta cutoff,
            // we don't need to search any captures.
            if best_value >= beta {
                // Return average of static eval and beta to avoid returning
                // values that are too far from the "true" evaluation.
                best_value = (best_value + beta) / 2;
                if tt_depth == -TT_DEPTH_OFFSET {
                    tt.insert(self.board.state.hash, Bound::None, Move::NONE, raw_value, best_value, TT_DEPTH_UNSEARCHED, t.ply, false);
                }
                return best_value;
            }

            // Raise alpha if our stand pat evaluation is better.
            alpha = alpha.max(best_value);
        };

        // -----------------------------------
        //             Moves loop
        // -----------------------------------
        t.prepare_next();

        let mut best_move = Move::NONE;
        let mut moves_exist = false;

        let mut mp = MovePicker::new(SearchType::Qs, in_check, tt_move, Eval::DRAW);
        while let Some(m) = mp.next(&self.board, t) {
            moves_exist = true;

            // -----------------------------------
            //              Pruning
            // -----------------------------------
            if !best_value.is_loss() {
                // Futility pruning in qsearch.
                // If our position + bonus can't reach alpha, and the move doesn't
                // win material according to SEE, skip it.
                if !self.board.in_check() && futility <= alpha && !self.board.see(m, Eval(1)) {
                    best_value = best_value.max(futility);
                    continue;
                }

                // SEE (Static Exchange Evaluation) pruning.
                // If a capture loses material, it's usually not worth considering
                // unless we're in a desperate position.
                if !self.board.see(m, Eval(sp_qs_margin())) {
                    continue;
                }
            }

            // -----------------------------------
            //             Make Move
            // -----------------------------------
            self.make_move(m, t);
            let v = -self.qsearch::<NT::Next>(t, tt, -beta, -alpha);
            self.undo_move(m, t);

            if t.stop {
                return Eval::DRAW;
            }

            // Update best move and alpha if we found an improvement.
            if v > best_value {
                best_value = v;

                if v > alpha {
                    best_move = m;

                    // Beta cutoff: position is too good.
                    if v >= beta {
                        break;
                    }

                    alpha = v;
                }
            }
        }

        // Checkmate detection.
        // If we're in check and have no legal moves, it's checkmate.
        if in_check && !moves_exist {
            return Eval::mated_in(t.ply);
        }

        // Adjust beta cutoff values to be more conservative.
        // This prevents qsearch from returning overly optimistic evaluations.
        if best_value >= beta && best_value.nonterminal() {
            best_value = (best_value + beta) / 2
        }

        // Save to TT.
        // If our eval is at least beta:
        //   We stopped searching after the beta cutoff, as we proved the position is too good.
        //   We don't know the exact value of the position, we just know it's at least beta.
        //
        // Otherwise, best eval < alpha (alpha did not improve):
        //   We searched all the moves we wanted to and none of them could improve our position.
        //   This means the best our position could be is best_eval.
        //
        // We can't use an exact bound as we don't know if we've searched all the moves.
        let bound = if best_value >= beta { Bound::Lower } else { Bound::Upper };
        tt.insert(self.board.state.hash, bound, best_move, raw_value, best_value, TT_DEPTH_QS, t.ply, tt_pv);

        best_value
    }
}
