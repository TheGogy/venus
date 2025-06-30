use std::time::Instant;

use crate::{interface::Engine, position::Position, threading::thread::Thread, time_management::timemanager::TimeManager, tt::table::TT};

// NOTE:  Make sure that bench depth is at least as high as the highest of any min depths in tuning.
const BENCH_DEPTH: i16 = 14;

impl Engine {
    /// Runs a benchmark of the engine on a number of positions.
    pub fn run_bench() {
        let mut total_nodes = 0;
        let mut total_time = 0;

        for fen in FENS {
            let tt = TT::default();
            let mut pos: Position = format!("fen {fen}").parse().unwrap();
            let mut thread = Thread::new(TimeManager::fixed_depth(BENCH_DEPTH));

            let start = Instant::now();
            pos.iterative_deepening::<false>(&mut thread, &tt);

            total_time += start.elapsed().as_micros();
            total_nodes += thread.nodes;
        }

        println!("{total_nodes} nodes {} nps", total_nodes * 1_000_000 / (total_time as u64).max(1))
    }
}

const FENS: &[&str] = &[
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "rn2kbnr/p1q1ppp1/1ppp3p/8/4B1b1/2P4P/PPQPPP2/RNB1K1NR w KQkq - 0 1",
    "rn1qkbnr/p3ppp1/1ppp2Qp/3B4/6b1/2P4P/PP1PPP2/RNB1K1NR b KQkq - 0 1",
    "rnkq1bnr/p3ppp1/1ppp3p/5b2/8/2PQ3P/PP1PPPB1/RNB1K1NR b KQ - 0 1",
    "rn1q1bnr/p2kppp1/2pp3p/1p3b2/1P6/2PQ3P/P2PPPB1/RNB1K1NR w KQ - 0 1",
    "rn3bnr/1kq1pp2/2pQ2Np/pp6/1P6/2PPPb1P/P4P2/RNB1KB2 w Q - 0 1",
    "rn3b1r/1kq1p3/2pQ1npp/Pp6/4b3/2PPP2P/P4P2/RNB1KB2 w Q - 0 1",
    "rn3b1r/1kq1p3/2p2npp/Pp3b2/7Q/2PPP2P/P4P2/RNB1KB2 b Q - 0 1",
    "r4b1r/1k2p3/n1p2npp/Pp3b2/5Q2/2PPP1qP/P4P2/RNB1KB2 w Q - 0 1",
    "4n3/3kr1b1/6pp/P3p3/4B1P1/2PPP1nP/1B1Nb3/R3K3 w - - 0 1",
    "4n3/6b1/4r1pp/P1k1p3/4B1P1/2PPP2P/R3b3/B3KN2 b - - 0 1",
    "r3k2r/p1pp1pb1/bn2Qnp1/2qPN3/1p2P3/2N5/PPPBBPPP/R3K2R b KQkq - 0 1",
    "2kr3r/p1ppqpb1/bn2Qnp1/3PN3/1p2P3/2N5/PPPBBPPP/R3K2R b KQ - 0 1",
    "3Qb1k1/1r2ppb1/pN1n2q1/Pp1Pp1Pr/4P2p/4BP2/4B1R1/1R5K b - - 0 1",
    "r2qk2r/ppp1ppbp/3p1np1/3Pn3/2P1P3/2N2B2/PP3PPP/R1BQK2R w KQkq - 0 1",
    "r2q1rk1/p1pp1pbp/np2pnp1/8/3P1B2/2P1PN1P/PP1N1PP1/R2QK2R w KQ - 0 1",
    "rnbqkb1r/ppp1pppp/8/8/4P3/2N2N2/PP1P1PPP/R1BQK2R w KQkq - 0 1",
    "r1bq1rk1/ppp1nppp/4n3/3p3Q/3P4/1BP1B3/PP1N2PP/R4RK1 w - - 0 1",
    "3q2k1/pb3p1p/4pbp1/2r5/PpN2N2/1P2P2P/5PP1/Q2R2K1 b - - 0 1",
    "2rqr1k1/1p3p1p/p2p2p1/P1nPb3/2B1P3/5P2/1PQ2NPP/R1R4K w - - 0 1",
    "8/n5bB/r7/P1k1p1pp/1R4PP/2PPP3/4b2N/B3K3 b - - 0 1",
    "7r/2p3k1/1pNpnqp1/1P1Bp3/p1P2r1P/P7/4R1K1/Q4R2 w - - 0 1",
    "8/5k2/1pnrp1p1/p1p4p/P6P/4R1PK/1P3P2/4R3 b - - 0 1",
    "1r5k/2pq2p1/3p3p/p1pP4/4QP2/PP1R3P/6PK/8 w - - 0 1",
    "8/n6B/r4b2/P1k1p1pp/1R4bP/2PPP3/8/B3KN2 w - - 0 1",
    "8/nR5B/r1k2b2/P3p1pp/6bP/2PPP3/8/B3KN2 b - - 0 1",
    "8/8/1p1k2p1/p1prp2p/P2n3P/6P1/1P1R1PK1/4R3 b - - 0 1",
    "r4qk1/6r1/1p4p1/2ppBbN1/1p5Q/P7/2P3PP/5RK1 w - - 0 1",
    "8/3p4/p1bk3p/Pp6/1Kp1PpPp/2P2P1P/2P5/5B2 b - - 0 1",
    "6R1/P2k4/r7/5N1P/r7/p7/7K/8 w - - 0 1",
    "8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - - 0 1",
    "r7/6k1/1p6/2pp1p2/7Q/8/p1P2K1P/8 w - - 0 1",
    "7K/8/k1P5/7p/8/8/8/8 w - - 0 1",
    "5k2/7R/4P2p/5K2/p1r2P1p/8/8/8 b - - 0 1",
    "4k3/5ppp/8/8/8/8/PPP5/3K4 w - - 0 1",
    "5B2/6P1/1p6/8/1N6/kP6/2K5/8 w - - 0 1",
    "8/8/8/8/5kp1/P7/8/1K1N4 w - - 0 1",
    "8/8/2P5/1Pr5/8/8/N7/k2K4 w - - 0 1",
    "8/8/1KP5/3r4/8/8/8/k7 w - - 0 1",
    "bbqnnrkr/pppppppp/8/8/8/8/PPPPPPPP/BBQNNRKR w HFhf - 0 1",
    "nqbnrkrb/pppppppp/8/8/8/8/PPPPPPPP/NQBNRKRB w KQkq - 0 1",
];
