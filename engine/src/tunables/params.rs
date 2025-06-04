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
    val_pawn:   i32 = 200, 150, 250, 5;
    val_knight: i32 = 780, 700, 900, 10;
    val_bishop: i32 = 820, 700, 900, 10;
    val_rook:   i32 = 1300, 1200, 1500, 10;
    val_queen:  i32 = 2500, 2400, 2600, 10;

    // Material scaling values.
    mat_scale_base: i32 = 700, 600, 900, 10;

    // Aspiration window.
    asp_window_default: i32 = 26, 10, 30, 1;
    asp_window_d_min:   i16 = 3, 2, 7, 1;

    // History bonuses.
    hist_bonus_max:  i16 = 1626, 800, 3200, 100;
    hist_bonus_base: i16 = 354, 100, 600, 25;
    hist_bonus_mult: i16 = 372, 100, 600, 25;

    // History maluses.
    hist_malus_max:  i16 = 1608, 800, 3200, 100;
    hist_malus_base: i16 = 372, 100, 600, 25;
    hist_malus_mult: i16 = 359, 100, 600, 25;

    // Continuation history scales (scaled up x1000).
    ch_scale_0: i32 = 1000, 500, 1500, 50;
    ch_scale_1: i32 = 1000, 500, 1500, 50;
    ch_scale_2: i32 = 1000, 500, 1500, 50;
    ch_scale_3: i32 = 1000, 500, 1500, 50;
    ch_scale_4: i32 = 1000, 500, 1500, 50;
    ch_scale_5: i32 = 1000, 500, 1500, 50;

    // transposition table.
    tt_replace_d_min: i16 = 4, 2, 6, 1;

    // extensions.
    ext_d_min:        i16 = 9, 6, 11, 1;
    ext_mult:         i16 = 128, 0, 5, 1;
    ext_double_e_min: i16 = 13, 0, 50, 1;
    ext_triple_e_min: i16 = 120, 0, 200, 1;

    // Late move reductions.
    lmr_d_min:      i16 = 2, 0, 6, 1;
    lmr_m_min:      i16 = 1, 0, 6, 1;
    lmr_root_bonus: i16 = 2, 0, 6, 1;

    // Late move reduction parameters (scaled up x1000).
    lmr_base: i32 = 839, 500, 2000, 100;
    lmr_mult: i32 = 1781, 1500, 4000, 100;

    // Late move reduction verifications.
    lmr_ver_e_min: i32 = 80, 40, 200, 2;

    // Reduction history metrics.
    lmr_quiet_div: i32 = 8500, 6000, 10000, 100;
    lmr_noisy_div: i32 = 6000, 4000, 10000, 100;

    // Reverse futility pruning.
    rfp_d_min:            i16 = 9, 5, 12, 1;
    rfp_mult:             i32 = 76, 40, 120, 5;
    rfp_improving_margin: i32 = 55, 25, 85, 5;
}
