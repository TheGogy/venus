use chess::types::board::Board;

fn main() {
    let mut b = Board::default();
    b.perft::<false>(6);
}
