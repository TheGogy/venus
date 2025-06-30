use chess::{MAX_MOVES, types::moves::Move};

use crate::{position::Position, threading::thread::Thread};

use super::{MovePicker, SearchType};

impl Position {
    /// Counts all the legal positions up to a given depth using the move picker.
    pub fn perftmp<const PRINT: bool>(&mut self, depth: usize) -> usize {
        let t = Thread::placeholder();
        self.perftmp_driver::<PRINT>(&t, depth)
    }

    fn perftmp_driver<const PRINT: bool>(&mut self, t: &Thread, depth: usize) -> usize {
        let mut total = 0;

        let mut ml = [Move::NONE; MAX_MOVES];
        let mut nb_moves = 0;
        let mut mp = MovePicker::new(SearchType::Pv, self.board.in_check(), Move::NONE);
        while let Some(m) = mp.next(&self.board, t) {
            ml[nb_moves] = m;
            nb_moves += 1;
        }

        if depth <= 1 {
            return nb_moves;
        }

        for m in ml[..nb_moves].iter() {
            self.board.make_move(*m);
            let n = self.perftmp_driver::<false>(t, depth - 1);
            self.board.undo_move(*m);

            total += n;

            if PRINT {
                println!("{} | {n}", m.to_uci(&self.board.castlingmask));
            }
        }

        total
    }
}

#[cfg(test)]
mod tests {
    use crate::position::Position;

    #[test]
    fn test_perftmp() {
        // Same test positions as perft.
        #[rustfmt::skip]
        const PERFT_TESTS: &[(&str, usize, usize)] = &[
            // Standard chess
            ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 119060324, 6),
            ("4k3/8/8/8/8/8/8/4K2R w K - 0 1 ", 764643, 6),
            ("4k3/8/8/8/8/8/8/R3K3 w Q - 0 1 ", 846648, 6),
            ("4k2r/8/8/8/8/8/8/4K3 w k - 0 1 ", 899442, 6),
            ("r3k3/8/8/8/8/8/8/4K3 w q - 0 1 ", 1001523, 6),
            ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 193690690, 5),
            ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 674624, 5),
            ("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", 15833292, 5),
            ("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8  ", 89941194, 5),
            ("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10 ", 164075551, 5),
            ("2K2r2/4P3/8/8/8/8/8/3k4 w - - 0 1", 3821001, 6),
            ("K1k5/8/P7/8/8/8/8/8 w - - 0 1", 2217, 6),
            ("8/8/1k6/2b5/2pP4/8/5K2/8 b - d3 0 1", 1440467, 6),
            ("3k4/3p4/8/K1P4r/8/8/8/8 b - - 0 1", 1134888, 6),
            ("5k2/8/8/8/4pP2/8/8/5RK1 b Q f3 0 9", 1050689, 6),
        ];

        for (fen, correct_count, depth) in PERFT_TESTS {
            let mut board: Position = format!("fen {fen}").parse().unwrap();
            println!("{fen}");
            let nodes = board.perftmp::<true>(*depth);
            assert_eq!(nodes, *correct_count);
        }
    }
}
