/// Make a full input bucket map from the half input buckets.
pub const fn make_bucket_map(half_buckets: [usize; 32], nb_buckets: usize) -> [usize; 64] {
    let mut bucket_map = [0; 64];

    let mut i = 0;
    while i < 64 {
        bucket_map[i] = half_buckets[(i / 8) * 4 + [0, 1, 2, 3, 3, 2, 1, 0][i % 8]];

        if i % 8 > 3 {
            bucket_map[i] += nb_buckets;
        }

        i += 1;
    }

    bucket_map
}
