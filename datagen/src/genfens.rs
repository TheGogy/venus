use chess::types::eval::Eval;
use engine::position::Position;
use fastrand::Rng;

/// Try this many times to find a move that passes the SEE threshold.
const RANDOM_MOVE_SEE_ATTEMPTS: usize = 8;
const SEE_THRESHOLD: Eval = Eval(-800);

pub fn gen_random_position(p: &mut Position, rng: &mut Rng, moves: usize) {
    p.reset();

    // Randomize starting side to prevent bias.
    for _ in 0..rng.usize(moves..(moves + 1)) {
        let mvs = p.board.gen_moves();

        // Game over: try again recursively.
        if mvs.is_empty() || p.board.is_draw(p.board.state.halfmoves) {
            return gen_random_position(p, rng, moves);
        }

        // Try to find a move that passes the SEE threshold.
        let mut move_found = false;
        for _ in 0..RANDOM_MOVE_SEE_ATTEMPTS {
            // SAFETY: We just checked if the move list is empty.
            let m = *rng.choice(mvs.iter()).unwrap();

            if p.board.see(m, SEE_THRESHOLD) {
                move_found = true;
                p.board.make_move(m);
                break;
            }
        }

        // Couldn't find a good move: Just pick a random one.
        if !move_found {
            // SAFETY: We just checked if the move list is empty.
            let m = *rng.choice(mvs.iter()).unwrap();
            p.board.make_move(m);
        }
    }

    // Wildly unbalanced position: try again.
    if p.evaluate().abs() > Eval(1000) {
        gen_random_position(p, rng, moves);
    }

    // Terminal position: try again.
    if !p.board.has_moves() {
        gen_random_position(p, rng, moves);
    }
}

pub fn genfens(amount: usize, seed: u64) {
    let mut rng = Rng::with_seed(seed);
    let mut p = Position::default();

    for _ in 0..amount {
        gen_random_position(&mut p, &mut rng, 8);
        println!("info string genfens {}", p.board.to_fen());
    }
}
