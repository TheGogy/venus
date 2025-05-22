use super::L1;

/// Get the index of the feature representing a given color, piece and square.
pub const fn feature_idx(c: usize, p: usize, s: usize) -> (usize, usize) {
    let w = c * 384 + p * 64 + s;
    let b = (1 ^ c) * 384 + p * 64 + (s ^ 56);

    (w * L1, b * L1)
}
