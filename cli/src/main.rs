use std::time::Instant;

use chess::{
    movegen::{ALL_MOVE, perft::perft},
    types::board::Board,
};

const BENCH_LIST: &[(&str, usize, usize)] = &[
    ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 6, 119060324),
    ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 5, 193690690),
    ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 7, 178633661),
    ("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", 6, 706045033),
    ("1k6/1b6/8/8/7R/8/8/4K2R b K - 0 1", 5, 1063513),
    ("3k4/3p4/8/K1P4r/8/8/8/8 b - - 0 1", 6, 1134888),
    ("8/8/4k3/8/2p5/8/B2P2K1/8 w - - 0 1", 6, 1015133),
    ("8/8/1k6/2b5/2pP4/8/5K2/8 b - d3 0 1", 6, 1440467),
    ("5k2/8/8/8/8/8/8/4K2R w K - 0 1", 6, 661072),
    ("3k4/8/8/8/8/8/8/R3K3 w Q - 0 1", 6, 803711),
    ("r3k2r/1b4bq/8/8/8/8/7B/R3K2R w KQkq - 0 1", 4, 1274206),
    ("r3k2r/8/3Q4/8/8/5q2/8/R3K2R b KQkq - 0 1", 4, 1720476),
    ("2K2r2/4P3/8/8/8/8/8/3k4 w - - 0 1", 6, 3821001),
    ("8/8/1P2K3/8/2n5/1q6/8/5k2 b - - 0 1", 5, 1004658),
    ("4k3/1P6/8/8/8/8/K7/8 w - - 0 1", 6, 217342),
    ("8/P1k5/K7/8/8/8/8/8 w - - 0 1", 6, 92683),
    ("K1k5/8/P7/8/8/8/8/8 w - - 0 1", 6, 2217),
    ("8/k1P5/8/1K6/8/8/8/8 w - - 0 1", 7, 567584),
    ("8/8/2k5/5q2/5n2/8/5K2/8 b - - 0 1", 4, 23527),
    ("4k3/8/8/8/8/8/8/4K2R w K - 0 1 ", 6, 764643),
    ("4k3/8/8/8/8/8/8/R3K3 w Q - 0 1 ", 6, 846648),
    ("4k2r/8/8/8/8/8/8/4K3 w k - 0 1 ", 6, 899442),
    ("r3k3/8/8/8/8/8/8/4K3 w q - 0 1 ", 6, 1001523),
    ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 5, 674624),
    ("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", 5, 15833292),
    ("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8  ", 5, 89941194),
    ("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10 ", 5, 164075551),
    ("2K2r2/4P3/8/8/8/8/8/3k4 w - - 0 1", 6, 3821001),
    ("K1k5/8/P7/8/8/8/8/8 w - - 0 1", 6, 2217),
    ("8/8/1k6/2b5/2pP4/8/5K2/8 b - d3 0 1", 6, 1440467),
    ("3k4/3p4/8/K1P4r/8/8/8/8 b - - 0 1", 6, 1134888),
    ("rbbknnqr/pppppppp/8/8/8/8/PPPPPPPP/RBBKNNQR w KQkq - 0 1", 6, 124381396),
    ("bnrkrnqb/pppppppp/8/8/8/8/PPPPPPPP/BNRKRNQB w KQkq - 0 1", 6, 146858295),
    ("nrbbqknr/pppppppp/8/8/8/8/PPPPPPPP/NRBBQKNR w KQkq - 0 1", 6, 97939069),
    ("bnrbnkrq/pppppppp/8/8/8/8/PPPPPPPP/BNRBNKRQ w KQkq - 0 1", 6, 145999259),
    ("rbknqnbr/pppppppp/8/8/8/8/PPPPPPPP/RBKNQNBR w KQkq - 0 1", 6, 126480040),
    ("qbrnnkbr/pppppppp/8/8/8/8/PPPPPPPP/QBRNNKBR w KQkq - 0 1", 6, 121613156),
];

fn main() {
    println!("==========  START BENCH  ===========");

    for (fen, depth, expected_nodes) in BENCH_LIST.iter() {
        let mut board: Board = fen.parse().unwrap();

        let start = Instant::now();
        let nodes = perft::<ALL_MOVE, false>(&mut board, *depth);
        let duration = start.elapsed();
        let perf: u128 = nodes as u128 / duration.as_micros();

        let status: &str = if nodes == *expected_nodes { "PASS" } else { "FAIL" };

        println!("Testing fen {fen}, expected nodes: {expected_nodes}, actual nodes: {nodes}, status: {status}, Mnps: {perf}")
    }
}
