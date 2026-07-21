use chess::types::{color::Color, piece::Piece, rank_file::File, square::Square};

use crate::arch::{BUCKET_MAP, FEATURES, HalfAcc, NB_OUTPUT_BUCKETS, NNUEData};

impl NNUEData {
    /// Get the weights for the given feature.
    pub fn feats_for(&self, mut ksq: Square, perspective: Color, p: Piece, c: Color, mut s: Square) -> &HalfAcc {
        const PIECE_STRIDE: usize = Square::NUM;
        const OPPONENT_STRIDE: usize = Square::NUM * Piece::NUM;
        const BUCKET_STRIDE: usize = FEATURES;

        if ksq.file() >= File::FE {
            ksq = ksq.fliph();
            s = s.fliph();
        }

        let bucket = input_bucket(ksq, perspective);
        let opponent = c.idx() ^ perspective.idx();

        let idx = bucket * BUCKET_STRIDE + opponent * OPPONENT_STRIDE + p.idx() * PIECE_STRIDE + s.relative(perspective).idx();

        &self.ftw[idx]
    }
}

/// Get the current input bucket to use.
pub const fn input_bucket(ksq: Square, c: Color) -> usize {
    BUCKET_MAP[ksq.relative(c).idx()]
}

/// Get the current output bucket to use.
pub const fn output_bucket(nb_pieces: usize) -> usize {
    const DIV: usize = usize::div_ceil(32, NB_OUTPUT_BUCKETS);
    (nb_pieces - 2) / DIV
}

/// Whether the king has changed position.
pub const fn king_changed(ks1: Square, ks2: Square, c: Color) -> bool {
    input_bucket(ks1, c) != input_bucket(ks2, c)
}
