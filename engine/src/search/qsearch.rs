use chess::{
    MAX_PLY,
    types::{eval::Eval, moves::Move},
};

use crate::{
    movepick::{MovePicker, SearchType},
    position::Position,
    threading::thread::Thread,
    tt::{entry::Bound, table::TT},
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
            return if in_check { Eval::DRAW } else { self.evaluate() };
        }

        // Stop searching if position is ruled as a draw.
        if self.board.is_draw(t.ply_from_null) {
            return Eval::DRAW;
        }

        // -----------------------------------
        //             TT lookup
        // -----------------------------------

        let mut tt_move = Move::NONE;
        let mut tt_value = Eval::NONE;
        let mut tt_eval = Eval::NONE;
        let mut tt_bound = Bound::None;
        let mut tt_depth = 0;
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
        if !NT::PV && tt_value.is_valid() && tt_depth > 0 && tt_bound.is_usable(tt_value, beta) {
            return tt_value;
        }

        // -----------------------------------
        //            Static Eval
        // -----------------------------------

        let mut best_eval;
        let futility;

        if in_check {
            // When in check, we must search all evasions - can't stand pat
            best_eval = -Eval::INFINITY;
            futility = -Eval::INFINITY;
            t.ss_mut().eval = -Eval::INFINITY;
        } else {
            // "Stand pat" evaluation: assume we can choose not to make any move
            // This is the key insight of qsearch - we can always choose to not capture.
            let mut v = if tt_eval.is_valid() { tt_eval } else { self.evaluate() };
            t.ss_mut().eval = v;

            // Futility pruning threshold for qsearch.
            // If our position + a reasonable bonus still can't reach alpha,
            // we can prune captures that don't improve the position significantly>
            futility = v + fp_qs_base();

            // Use TT score if it's more accurate than static eval.
            if tt_value.is_valid() && tt_bound.is_usable(tt_value, v) {
                v = tt_value
            }

            // Beta cutoff from stand pat.
            // If our current position is already good enough to cause a beta cutoff,
            // we don't need to search any captures.
            if v >= beta {
                // Return average of static eval and beta to avoid returning
                // values that are too far from the "true" evaluation.
                return (v + beta) / 2;
            }

            // Raise alpha if our stand pat evaluation is better.
            alpha = alpha.max(v);

            best_eval = v;
        };

        // -----------------------------------
        //             Moves loop
        // -----------------------------------
        let mut best_move = Move::NULL;
        let mut moves_exist = false;

        let mut mp = MovePicker::new(SearchType::Qs, in_check, tt_move);
        while let Some(m) = mp.next(&self.board, t) {
            moves_exist = true;

            // -----------------------------------
            //              Pruning
            // -----------------------------------
            if !best_eval.is_loss() {
                // Futility pruning in qsearch.
                // If our position + bonus can't reach alpha, and the move doesn't
                // win material according to SEE, skip it.
                if !self.board.in_check() && futility <= alpha && !self.board.see(m, Eval(1)) {
                    best_eval = best_eval.max(futility);
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
            if v > best_eval {
                best_eval = v;

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
        if best_eval >= beta && best_eval.nonterminal() {
            best_eval = (best_eval + beta) / 2
        }

        // Save to TT.
        // If the best eval does not exceed beta,
        if !t.stop {
            let tt_flag = if best_eval >= beta {
                // Position is at least as good as beta.
                Bound::Lower
            } else {
                // All moves failed to raise alpha.
                Bound::Upper
            };

            tt.insert(self.board.state.hash, tt_flag, best_move, t.ss().eval, best_eval, 0, t.ply, tt_pv);
        }
        best_eval
    }
}
