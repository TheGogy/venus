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
    // Material scaling values.
    ms_base: i32 = 715, 600, 900, 10;
    ms_knight: i32 = 446, 400, 500, 5;
    ms_bishop: i32 = 459, 400, 500, 5;
    ms_rook:   i32 = 705, 600, 800, 10;
    ms_queen:  i32 = 1313, 1200, 1400, 10;

    // Aspiration window.
    asp_window_default: i32 = 24, 10, 30, 1;
    asp_window_d_min:   i16 = 4, 2, 7, 1;

    // History bonuses.
    hist_bonus_max:  i16 = 1567, 800, 3200, 100;
    hist_bonus_base: i16 = 356, 100, 600, 25;
    hist_bonus_mult: i16 = 366, 100, 600, 25;

    // History maluses.
    hist_malus_max:  i16 = 1680, 800, 3200, 100;
    hist_malus_base: i16 = 356, 100, 600, 25;
    hist_malus_mult: i16 = 366, 100, 600, 25;

    // History divisors.
    hist_quiet_div: i32 = 8867, 7000, 10000, 150;
    hist_noisy_div: i32 = 6329, 5000, 8000, 150;

    // Correction history weights. (scaled up x1024).
    hist_corr_pawn: i32 = 80, 60, 100, 2;
    hist_corr_other: i32 = 100, 80, 120, 2;

    // transposition table.
    tt_replace_d_min: i16 = 5, 2, 6, 1;

    // Probcut.
    pc_beta_base:          i32 = 143, 120, 200, 5;
    pc_beta_non_improving: i32 = 55, 30, 80, 4;

    // extensions.
    ext_d_min: i16 = 8, 5, 10, 1;
    ext_mult:  i16 = 2, 1, 4, 1;
    ext_double: i32 = 15, 10, 30, 1;

    // Late move reduction table parameters (scaled up x1024).
    lmr_base: i32 = 887, 500, 2000, 100;
    lmr_mult: i32 = 2003, 1500, 4000, 100;

    // Late move reductions.
    lmr_d_min: i16 = 2, 1, 4, 1;
    lmr_m_min: usize = 2, 1, 4, 1;

    // Late move reduction verifications.
    lmr_ver_e_min: i32 = 42, 30, 50, 1;

    // Reduction history metrics.
    lmr_quiet_div: i32 = 8439, 6000, 10000, 100;
    lmr_noisy_div: i32 = 5910, 4000, 10000, 100;

    // Reverse futility pruning.
    rfp_d_max:            i16 = 8, 5, 12, 1;
    rfp_mult:             i32 = 82, 40, 120, 5;
    rfp_improving_margin: i32 = 59, 25, 85, 5;

    // Null move pruning.
    nmp_d_min:            i16 = 3, 1, 4, 1;
    nmp_improving_margin: i32 = 68, 40, 100, 5;
    nmp_base:             i16 = 5, 2, 7, 1;
    nmp_factor:           i16 = 3, 2, 8, 1;

    // Internal iterative reductions.
    iir_d_min:  i16 = 2, 1, 4, 1;

    // Razoring.
    razoring_d_max:  i16 = 7, 2, 10, 1;
    razoring_e_max:  i32 = 1896, 1000, 3000, 100;
    razoring_d_mult: i32 = 346, 300, 800, 25;

    // History pruning.
    hp_d_min: i16 = 2, 1, 5, 1;
    hp_s_min: i32 = 5000, 3500, 6000, 100;

    // Futility pruning.
    fp_base:  i32 = 80, 50, 100, 2;
    fp_mult:  i32 = 91, 50, 100, 2;
    fp_d_min: i16 = 5, 3, 8, 1;

    // Futility pruning for qsearch.
    fp_qs_base: i32 = 353, 300, 400, 5;

    // Late move pruning.
    lmp_base:  i16 = 2, 2, 8, 1;
    lmp_d_min: i16 = 8, 5, 12, 1;

    // SEE pruning.
    sp_noisy_margin: i32 = -17, -40, 0, 5;
    sp_quiet_margin: i32 = -70, -120, -30, 5;
    sp_d_min:        i16 = 10, 6, 14, 1;
    sp_qs_margin:    i32 = -33, -50, -10, 2;
}
