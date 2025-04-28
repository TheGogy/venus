/// Get the next randomly generated number.
/// https://en.wikipedia.org/wiki/Xorshift#xorshift*
pub const fn next_rng(state: u64) -> u64 {
    let mut state = state;
    state ^= state >> 12;
    state ^= state << 25;
    state ^= state >> 27;
    state.wrapping_mul(0x2545F4914F6CDD1D)
}
