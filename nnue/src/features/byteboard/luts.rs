use utils::{cfor, memory::Align64};

#[rustfmt::skip]
pub static PIECE_BIT_LUT: Align64<[u8; 16]> = Align64([
    0b0000_0001, 0b0000_0010, // Pawn
    0b0000_0100, 0b0000_0100, // Knight
    0b0000_1000, 0b0000_1000, // Bishop
    0b0001_0000, 0b0001_0000, // Rook
    0b0010_0000, 0b0010_0000, // Queen
    0b0100_0000, 0b0100_0000, // King
    0, 0, 0, 0, // Pad
]);

#[rustfmt::skip]
pub const OUTGOING_THREATS_MASK: [u64; 12] = [
    0x02_00_00_00_00_00_02_00, 0x00_00_02_00_02_00_00_00, // Pawn
    0x01_01_01_01_01_01_01_01, 0x01_01_01_01_01_01_01_01, // Knight
    0xFE_00_FE_00_FE_00_FE_00, 0xFE_00_FE_00_FE_00_FE_00, // Bishop
    0x00_FE_00_FE_00_FE_00_FE, 0x00_FE_00_FE_00_FE_00_FE, // Rook
    0xFE_FE_FE_FE_FE_FE_FE_FE, 0xFE_FE_FE_FE_FE_FE_FE_FE, // Queen
    0x0,                       0x0,                       // King
];

pub const NON_KNIGHTS_MASK: u64 = 0xFE_FE_FE_FE_FE_FE_FE_FE;

pub static INCOMING_THREATS_LUT: Align64<[u8; 64]> = {
    const KNIGHT: u8 = 0b0000_0100; // Knight
    const ORTH: u8 = 0b0011_0000; // Queen | Rook
    const DIAG: u8 = 0b0010_1000; // Queen | Bishop
    const ORTH_NEAR: u8 = 0b0111_0000; // King, Queen, Rook
    const WPWN_NEAR: u8 = 0b0110_1001; // King, Queen, Bishop, White pawn
    const BPWN_NEAR: u8 = 0b0110_1010; // King, Queen, Bishop, Black pawn
    Align64([
        KNIGHT, ORTH_NEAR, ORTH, ORTH, ORTH, ORTH, ORTH, ORTH, // North
        KNIGHT, BPWN_NEAR, DIAG, DIAG, DIAG, DIAG, DIAG, DIAG, // NorthEast
        KNIGHT, ORTH_NEAR, ORTH, ORTH, ORTH, ORTH, ORTH, ORTH, // East
        KNIGHT, WPWN_NEAR, DIAG, DIAG, DIAG, DIAG, DIAG, DIAG, // SouthEast
        KNIGHT, ORTH_NEAR, ORTH, ORTH, ORTH, ORTH, ORTH, ORTH, // South
        KNIGHT, WPWN_NEAR, DIAG, DIAG, DIAG, DIAG, DIAG, DIAG, // SouthWest
        KNIGHT, ORTH_NEAR, ORTH, ORTH, ORTH, ORTH, ORTH, ORTH, // West
        KNIGHT, BPWN_NEAR, DIAG, DIAG, DIAG, DIAG, DIAG, DIAG, // NorthWest
    ])
};

pub static INCOMING_SLIDERS_LUT: Align64<[u8; 64]> = {
    const ORTH: u8 = 0b0011_0000;
    const DIAG: u8 = 0b0010_1000;
    const NULL: u8 = 0b1000_0000;
    Align64([
        NULL, ORTH, ORTH, ORTH, ORTH, ORTH, ORTH, ORTH, // North
        NULL, DIAG, DIAG, DIAG, DIAG, DIAG, DIAG, DIAG, // NorthEast
        NULL, ORTH, ORTH, ORTH, ORTH, ORTH, ORTH, ORTH, // East
        NULL, DIAG, DIAG, DIAG, DIAG, DIAG, DIAG, DIAG, // SouthEast
        NULL, ORTH, ORTH, ORTH, ORTH, ORTH, ORTH, ORTH, // South
        NULL, DIAG, DIAG, DIAG, DIAG, DIAG, DIAG, DIAG, // SouthWest
        NULL, ORTH, ORTH, ORTH, ORTH, ORTH, ORTH, ORTH, // West
        NULL, DIAG, DIAG, DIAG, DIAG, DIAG, DIAG, DIAG, // NorthWest
    ])
};

/// Ray slot that falls off the board.
pub const OFF_BOARD: u8 = 0x40;

#[expect(clippy::cast_possible_truncation)]
pub static PERM_INDEX_LUT: [Align64<[u8; 64]>; 64] = {
    let offsets = [
        0x1F, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, // North
        0x21, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, // NorthEast
        0x12, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, // East
        0xF2, 0xF1, 0xE2, 0xD3, 0xC4, 0xB5, 0xA6, 0x97, // SouthEast
        0xE1, 0xF0, 0xE0, 0xD0, 0xC0, 0xB0, 0xA0, 0x90, // South
        0xDF, 0xEF, 0xDE, 0xCD, 0xBC, 0xAB, 0x9A, 0x89, // SouthWest
        0xEE, 0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0xF9, // West
        0x0E, 0x0F, 0x1E, 0x2D, 0x3C, 0x4B, 0x5A, 0x69, // NorthWest
    ];

    let mut perms = [Align64([0; 64]); 64];

    cfor!(let mut sq = 0; sq < 64; sq += 1; {
        let wide_focus = sq + (sq & 0x38);
        cfor!(let mut i = 0; i < 64; i += 1; {
            let wide_result = offsets[i] + wide_focus;
            let result = ((wide_result & 0x70) >> 1) | (wide_result & 0x07);
            let valid = (wide_result & 0x88) == 0;
            perms[sq].0[i] = if valid { result as u8 } else { OFF_BOARD };
        });
    });

    perms
};
