use crate::types::board::Board;

/// Counts all the legal positions up to a given depth.
pub fn perft<const PRINT: bool>(b: &mut Board, depth: usize) -> usize {
    let mut total = 0;

    let ml = b.gen_moves::<true>();

    // Base case: just count leaf nodes.
    if !PRINT && depth <= 1 {
        return ml.len();
    }

    for m in ml.iter() {
        let n = if depth == 1 {
            1
        } else {
            b.make_move(*m);
            let n = perft::<false>(b, depth - 1);
            b.undo_move(*m);
            n
        };

        total += n;

        if PRINT && n > 0 {
            println!("{m} | {n}");
        }
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perft() {
        #[rustfmt::skip]
        const PERFT_TESTS: [(&str, usize, usize); 20] = [
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

            // FRC
            // See: http://www.open-aurec.com/wbforum/viewtopic.php?t=1404
            ("rbbknnqr/pppppppp/8/8/8/8/PPPPPPPP/RBBKNNQR w KQkq - 0 1", 124381396, 6),
            ("bnrkrnqb/pppppppp/8/8/8/8/PPPPPPPP/BNRKRNQB w KQkq - 0 1", 146858295, 6),
            ("nrbbqknr/pppppppp/8/8/8/8/PPPPPPPP/NRBBQKNR w KQkq - 0 1", 97939069, 6),
            ("bnrbnkrq/pppppppp/8/8/8/8/PPPPPPPP/BNRBNKRQ w KQkq - 0 1", 145999259, 6),
            ("rbknqnbr/pppppppp/8/8/8/8/PPPPPPPP/RBKNQNBR w KQkq - 0 1", 126480040, 6),
            ("qbrnnkbr/pppppppp/8/8/8/8/PPPPPPPP/QBRNNKBR w KQkq - 0 1", 121613156, 6),
        ];

        for (fen, correct_count, depth) in PERFT_TESTS {
            let mut board: Board = fen.parse().unwrap();
            println!("{fen}");
            let nodes = perft::<true>(&mut board, depth);
            assert_eq!(nodes, correct_count);
        }
    }
}
