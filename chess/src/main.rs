use chess::types::board::Board;

/// Simple perft benchmark.
/// TODO: Make full benchmark suite.
fn main() {
    let mut b = Board::default();
    b.perft::<false>(6);
}
