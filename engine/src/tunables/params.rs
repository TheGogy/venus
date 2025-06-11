use crate::init_tunables;

// See [init_tunables].
//
// https://github.com/AndyGrant/OpenBench/wiki/SPSA-Tuning-Workloads
//
// Use the following naming convention:
//
// High level -> low level:
//      Each family of values should start the same.
//      e.g "val_pawn" and "val_knight".
//
// d_min => The minimum depth that this thing happens.
// d_max => The maximum depth that this thing happens.
//
// e_min => The minimum eval that this thing happens.
// e_max => The maximum eval that this thing happens.
//
// m_min => The minimum number of moves tried that this thing happens.
// m_max => The maximum number of moves tried that this thing happens.
//
// base => The base value (combined with a multiplier).
// mult => The multiplier (combined with a base value).
init_tunables! {
    // Piece values.
    val_pawn:   i32 = 199, 150, 250, 5;
    val_knight: i32 = 778, 700, 900, 10;
    val_bishop: i32 = 803, 700, 900, 10;
    val_rook:   i32 = 1297, 1200, 1500, 10;
    val_queen:  i32 = 2490, 2400, 2600, 10;

    // Material scaling values.
    mat_scale_base: i32 = 700, 600, 900, 10;

    // Aspiration window.
    asp_window_default: i32 = 26, 10, 30, 1;
    asp_window_d_min:   i16 = 3, 2, 7, 1;

    // History bonuses.
    hist_bonus_max:  i16 = 1556, 800, 3200, 100;
    hist_bonus_base: i16 = 356, 100, 600, 25;
    hist_bonus_mult: i16 = 386, 100, 600, 25;

    // History maluses.
    hist_malus_max:  i16 = 1609, 800, 3200, 100;
    hist_malus_base: i16 = 371, 100, 600, 25;
    hist_malus_mult: i16 = 358, 100, 600, 25;

    // History divisors.
    hist_quiet_div: i32 = 8928, 7000, 10000, 150;
    hist_noisy_div: i32 = 6195, 5000, 8000, 150;

    // Continuation history scales (scaled up x1024).
    ch_scale_0: i32 = 986, 500, 1500, 50;
    ch_scale_1: i32 = 996, 500, 1500, 50;
    ch_scale_2: i32 = 1038, 500, 1500, 50;
    ch_scale_3: i32 = 956, 500, 1500, 50;
    ch_scale_4: i32 = 982, 500, 1500, 50;
    ch_scale_5: i32 = 963, 500, 1500, 50;

    // transposition table.
    tt_replace_d_min: i16 = 2, 2, 6, 1;

    // extensions.
    ext_d_min:        i16 = 7, 6, 11, 1;
    ext_mult:         i16 = 124, 60, 200, 8;
    ext_double_e_min: i16 = 12, 0, 50, 1;
    ext_triple_e_min: i16 = 119, 0, 200, 1;
    ext_double_max: usize = 5, 1, 8, 1;

    // Late move reduction parameters (scaled up x1024).
    lmr_base: i32 = 888, 500, 2000, 100;
    lmr_mult: i32 = 2034, 1500, 4000, 100;

    // Late move reduction verifications.
    lmr_ver_e_min: i32 = 77, 40, 200, 2;

    // Reduction history metrics.
    lmr_quiet_div: i32 = 8568, 6000, 10000, 100;
    lmr_noisy_div: i32 = 5798, 4000, 10000, 100;

    // Reverse futility pruning.
    rfp_d_min:            i16 = 8, 5, 12, 1;
    rfp_mult:             i32 = 85, 40, 120, 5;
    rfp_improving_margin: i32 = 54, 25, 85, 5;

    // Null move pruning.
    nmp_d_min:            i16 = 1, 1, 4, 1;
    nmp_improving_margin: i32 = 68, 40, 100, 5;
    nmp_base:             i16 = 5, 2, 7, 1;
    nmp_factor:           i16 = 3, 2, 8, 1;

    // Internal iterative reductions.
    iir_pv_d_min:  i16 = 5, 3, 7, 1;
    iir_opv_d_min: i16 = 7, 5, 10, 1;

    // Razoring.
    razoring_d_max:  i16 = 6, 2, 10, 1;
    razoring_e_max:  i32 = 2000, 1000, 3000, 100;
    razoring_d_mult: i32 = 407, 300, 800, 25;

    // Futility pruning.
    fp_base:  i32 = 77, 50, 100, 2;
    fp_mult:  i32 = 52, 30, 80, 2;
    fp_d_min: i16 = 6, 3, 8, 1;

    // Futility pruning for qsearch.
    fp_qs_base: i32 = 350, 300, 400, 5;

    // Late move pruning.
    lmp_base:  i16 = 3, 2, 8, 1;
    lmp_d_min: i16 = 10, 5, 12, 1;

    // SEE pruning.
    sp_noisy_margin: i32 = -13, -40, 0, 5;
    sp_quiet_margin: i32 = -70, -100, -30, 5;
    sp_d_min:        i16 = 10, 6, 14, 1;
    sp_qs_margin: i32 = -32, -10, -50, 2;
}
