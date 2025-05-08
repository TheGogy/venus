use crate::init_tunables;

// See [init_tunables].
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
// base => The base value (combined with a multiplier).
// mult => The multiplier (combined with a base value).
init_tunables! {
    // Piece values.
    val_pawn:   i32 = 200, 150, 250, 5;
    val_knight: i32 = 780, 700, 900, 10;
    val_bishop: i32 = 820, 700, 900, 10;
    val_rook:   i32 = 1300, 1200, 1500, 10;
    val_queen:  i32 = 2500, 2400, 2600, 10;

    // Aspiration window.
    asp_window_default: i32 = 26, 10, 30, 1;
    asp_window_d_min: usize = 3, 2, 7, 1;

    // History bonuses.
    hist_bonus_max:  i16 = 1626, 800, 3200, 100;
    hist_bonus_base: i16 = 354, 100, 600, 25;
    hist_bonus_mult: i16 = 372, 100, 600, 25;

    // History maluses.
    hist_malus_max:  i16 = 1608, 800, 3200, 100;
    hist_malus_base: i16 = 372, 100, 600, 25;
    hist_malus_mult: i16 = 359, 100, 600, 25;

    // transposition table.
    tt_replace_d_min: usize = 4, 2, 6, 1;

    // Singular extensions.
    se_d_min: usize = 9, 6, 11, 1;
    se_mult: usize = 2, 0, 5, 1;

}
