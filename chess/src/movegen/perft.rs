use crate::types::board::Board;

use super::ALL_MOVE;

/// Counts all the legal positions up to a given depth.
pub fn perft<const PRINT: bool>(b: &mut Board, depth: usize) -> usize {
    let mut total = 0;

    let ml = b.gen_moves::<ALL_MOVE>();

    // Base case: just count leaf nodes.
    if !PRINT && depth <= 1 {
        return ml.len();
    }

    let is_leaf = depth == 2;

    for m in ml.iter() {
        b.make_move(*m);
        let n = if is_leaf { b.gen_moves::<ALL_MOVE>().len() } else { perft::<false>(b, depth - 1) };
        b.undo_move(*m);

        total += n;

        if PRINT && n > 0 {
            println!("{} | {n}", m.to_uci(&b.castlingmask));
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

            // FRC starting positions
            // See: http://www.open-aurec.com/wbforum/viewtopic.php?t=1404
            ("rbbknnqr/pppppppp/8/8/8/8/PPPPPPPP/RBBKNNQR w KQkq - 0 1", 124381396, 6),
            ("bnrkrnqb/pppppppp/8/8/8/8/PPPPPPPP/BNRKRNQB w KQkq - 0 1", 146858295, 6),
            ("nrbbqknr/pppppppp/8/8/8/8/PPPPPPPP/NRBBQKNR w KQkq - 0 1", 97939069, 6),
            ("bnrbnkrq/pppppppp/8/8/8/8/PPPPPPPP/BNRBNKRQ w KQkq - 0 1", 145999259, 6),
            ("rbknqnbr/pppppppp/8/8/8/8/PPPPPPPP/RBKNQNBR w KQkq - 0 1", 126480040, 6),
            ("qbrnnkbr/pppppppp/8/8/8/8/PPPPPPPP/QBRNNKBR w KQkq - 0 1", 121613156, 6),

            // FRC test positions
            ("8/3k4/8/8/8/8/8/rR2K3 w Q - 0 1", 7137508, 6),
            ("Rr2k3/8/8/8/8/8/8/rR2K3 w Qq - 0 1", 46081241, 6),
            ("2k5/8/8/8/b7/8/8/2K3R1 w - - 0 1", 6578528, 6),
            ("3k4/8/8/8/8/8/8/rRK5 w - - 0 1", 3191684, 6),
            ("1rkr4/8/8/8/8/8/8/1RKR4 w KQkq - 0 1", 66969143, 6),
            ("3k4/3q1q2/8/8/8/4Q3/3P4/1R1K2R1 w KQ - 0 1", 2938241633, 6),
            ("1b1qbkrn/1prp1pp1/pn5p/2p1pB2/Q1PP4/1N6/PP2PPPP/2R1BKRN w KQk - 2 9", 1648762553, 6),
            ("1rkb1qr1/pppp2pp/1n2p1n1/3b1p2/3N3P/P2P1P2/1PP1P1P1/1RKBBQRN w KQkq - 3 9", 1042669941, 6),
            ("1b1r1krb/ppp1np2/qn1p2pp/3Bp3/2P1P1PP/1N1P4/PP3P2/1BNRQKR1 w KQkq - 0 9", 1169912833, 6),
        ];

        for (fen, correct_count, depth) in PERFT_TESTS {
            let mut board: Board = fen.parse().unwrap();
            println!("{fen}");
            let nodes = perft::<true>(&mut board, *depth);
            assert_eq!(nodes, *correct_count);
        }
    }
}
