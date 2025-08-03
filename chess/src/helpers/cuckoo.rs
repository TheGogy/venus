use crate::{
    tables::{atk_by_type_const, sliding_piece::between},
    types::{
        board::Board,
        color::Color,
        moves::{Move, MoveFlag},
        piece::{CPiece, Piece},
        square::Square,
        zobrist::{COLOR_KEY, PIECE_KEYS},
    },
};

impl Board {
    /// Detect if the board has an upcoming repetition.
    pub fn upcoming_repetition(&self, ply: usize) -> bool {
        let end = self.state.halfmoves.min(self.history.len());

        if end < 3 {
            return false;
        }

        let occ = self.occ();
        let curr_key = self.state.hash.key;

        for i in (3..=end).step_by(2) {
            let move_key = curr_key ^ self.prev_key(i);

            let mut key = h1(move_key);

            // Check if key matches.
            if CUCKOO.keys[key] != move_key {
                key = h2(move_key);
            }
            if CUCKOO.keys[key] != move_key {
                continue;
            }

            // We have found a match.
            let mov = CUCKOO.moves[key];
            let (src, dst) = (mov.src(), mov.dst());

            // Check if move is blocked by some piece (except for captured piece).
            if !((between(src, dst) ^ dst.bb()) & occ).is_empty() {
                continue;
            }

            // Check if we are looking for repetitions after the root.
            if ply > i {
                return true;
            }

            let pc = if occ.has(src) { self.pc_at(src) } else { self.pc_at(dst) };

            if pc.color() != self.stm {
                continue;
            }

            // We need one more repetition at and before the root.
            for j in (i + 4..=end).step_by(2) {
                if self.prev_key(j) == self.prev_key(i) {
                    return true;
                }
            }
        }

        false
    }

    /// Get the key of the previous position.
    pub fn prev_key(&self, i: usize) -> u64 {
        assert!(i <= self.history.len());
        self.history[self.history.len() - i].hash.key
    }
}

/// Cuckoo table.
/// This allows us to detect upcoming repetitions.
struct CuckooTable {
    keys: [u64; 8192],
    moves: [Move; 8192],
}

const fn h1(k: u64) -> usize {
    (k & 0x1fff) as usize
}

const fn h2(k: u64) -> usize {
    ((k >> 16) & 0x1fff) as usize
}

static CUCKOO: CuckooTable = {
    let mut ct = CuckooTable { keys: [0; 8192], moves: [Move::NONE; 8192] };

    let mut c = 0; // White ..= Black
    while c < 2 {
        let mut p = 1; // Knight ..= King
        while p < 6 {
            let pc = CPiece::create(Color::from_raw(c), Piece::from_raw(p));

            let mut x = 0;
            while x < 64 {
                let mut y = x + 1;
                while y < 64 {
                    let sq_x = Square::from_raw(x);
                    let sq_y = Square::from_raw(y);

                    if atk_by_type_const(pc.pt(), sq_x).has(sq_y) {
                        let mut mv = Move::new(sq_x, sq_y, MoveFlag::Normal);
                        let mut key = PIECE_KEYS[pc.idx()][x as usize] ^ PIECE_KEYS[pc.idx()][y as usize] ^ COLOR_KEY;

                        let mut idx = h1(key);

                        loop {
                            std::mem::swap(&mut ct.keys[idx], &mut key);
                            std::mem::swap(&mut ct.moves[idx], &mut mv);

                            if mv.is_none() {
                                break;
                            }

                            idx = if idx == h1(key) { h2(key) } else { h1(key) };
                        }
                    }
                    y += 1;
                }
                x += 1;
            }
            p += 1;
        }
        c += 1;
    }

    ct
};

#[cfg(test)]
mod tests {
    use crate::helpers::cuckoo::CUCKOO;

    #[test]
    fn test_cuckoo_table_initialization() {
        let mut found_entries = 0;
        let mut valid_moves = 0;

        for i in 0..8192 {
            if CUCKOO.keys[i] != 0 {
                found_entries += 1;

                let mov = CUCKOO.moves[i];
                if !mov.is_none() {
                    valid_moves += 1;

                    // Verify the move is valid
                    assert!(!mov.is_none());
                    assert_ne!(mov.src(), mov.dst());
                }
            }
        }

        assert_eq!(found_entries, valid_moves);
        assert_eq!(found_entries, 3668);
    }
}
