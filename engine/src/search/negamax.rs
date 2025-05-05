use chess::types::{eval::Eval, moves::Move};

use crate::{position::pos::Pos, threading::thread::Thread};

use super::{NodeType, pv::PVLine};

impl Pos {
    /// Negamax search function.
    /// This performs the majority of the searching, then drops into qsearch at the end.
    pub fn negamax<NT: NodeType>(&mut self, t: &mut Thread, pv: &mut PVLine, mut alpha: Eval, mut beta: Eval, mut depth: usize) -> Eval {
        if t.should_stop() {
            t.stop = true;
            return Eval::DRAW;
        }

        // Base case: depth = 0
        if depth == 0 {
            return self.evaluate();
        }

        pv.clear();

        t.seldepth = if NT::RT { 0 } else { t.seldepth.max(t.ply) };

        if !NT::RT {
            alpha = alpha.max(Eval::mated_in(t.ply));
            beta = beta.min(Eval::mate_in(t.ply + 1));

            if alpha >= beta {
                return alpha;
            }

            if self.board.is_draw(t.ply_from_null) {
                return Eval::DRAW;
            }
        }

        let mut best_eval = -Eval::INFINITY;
        let mut best_move = Move::NULL;

        let mut caps_tried = Vec::with_capacity(24);
        let mut quiets_tried = Vec::with_capacity(24);

        let child_pv = &mut PVLine::default();

        let in_check = self.board.in_check();

        let mut mp = match self.init_movepicker::<true>(None) {
            Some(mp) => mp,
            None => {
                return if in_check { Eval::mated_in(t.ply) } else { Eval::DRAW };
            }
        };

        while let Some((m, _)) = mp.next(&self.board, t) {
            assert!(m.is_valid());

            let start_nodes = t.nodes;
            let is_quiet = m.flag().is_quiet();

            self.make_move(m, t);

            let v = -self.negamax::<NT::Next>(t, child_pv, -beta, -alpha, depth - 1);

            self.undo_move(m, t);

            if t.stop {
                return Eval::DRAW;
            }

            if NT::RT {
                t.clock.update_node_count(m, t.nodes - start_nodes);
            }

            if v > best_eval {
                best_eval = v;

                if v > alpha {
                    best_move = m;
                    alpha = best_eval;

                    if NT::PV {
                        pv.update(m, child_pv);
                    }
                }

                if v >= beta {
                    alpha = beta;
                    t.update_tables(m, depth, &self.board, quiets_tried, caps_tried);
                    break;
                }
            }

            // Add move to history
            if is_quiet {
                quiets_tried.push(m);
            } else if m.flag().is_cap() {
                caps_tried.push(m);
            }
        }

        alpha
    }
}
