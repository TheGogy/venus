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
        let child_pv = &mut PVLine::default();

        // TODO: Movepicking
        let moves = self.board.gen_moves::<true>();

        for &m in moves.iter() {
            self.board.make_move(m);
            t.move_made();

            let v = -self.negamax::<NT::Next>(t, child_pv, -beta, -alpha, depth - 1);

            self.board.undo_move(m);
            t.move_undo();

            if t.stop {
                return Eval::DRAW;
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
                    break;
                }
            }
        }

        if best_move.is_null() {
            return if self.board.in_check() { Eval::mate_in(t.ply) } else { Eval::DRAW };
        }

        alpha
    }
}
