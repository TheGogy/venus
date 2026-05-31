use arrayvec::ArrayVec;
use chess::types::{
    bitboard::Bitboard,
    board::{Board, BoardState},
    castling::CastlingRights,
    color::Color,
    piece::{CPiece, Piece},
    square::Square,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MarlinWDL {
    Win = 2,
    Loss = 0,

    #[default]
    Draw = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct MarlinFmt {
    pub occ: u64,
    pub pcs: [u8; 16],
    pub stm_epsq: u8,
    pub halfmoves: u8,
    pub fullmoves: u16,
    pub eval: i16,
    pub wdl: MarlinWDL,

    pad: u8,
}

const _: () = assert!(std::mem::size_of::<MarlinFmt>() == 32);

impl From<&Board> for MarlinFmt {
    #[allow(clippy::cast_possible_truncation)]
    fn from(b: &Board) -> Self {
        let occ = b.occ();
        let mut packed = Self {
            occ: occ.0,
            pcs: [0; 16],
            stm_epsq: ((b.stm as u8) << 7) | (b.state.epsq as u8),
            halfmoves: (b.state.halfmoves as u8).to_le(),
            fullmoves: (b.state.fullmoves as u16).to_le(),
            eval: 0,
            wdl: MarlinWDL::Draw,

            pad: 0,
        };

        let mut i = 0;
        occ.bitloop(|s| {
            let pc = b.pc_at(s);

            // Marlin uses unmoved rooks to handle castling, regular pieces for everything else.
            let mut marlin_pc = if pc.pt() == Piece::Rook && b.castlingmask.mask[s.idx()] != CastlingRights::NONE {
                Self::UNMOVED_ROOK
            } else {
                pc.pt() as u8
            };

            // furthest left bit set for color.
            marlin_pc |= (pc.color() as u8) << 3;

            packed.pcs[i / 2] |= marlin_pc << ((i % 2) * 4);
            i += 1;
        });

        packed
    }
}

impl MarlinFmt {
    // Marlinformat represents castling with unmoved rooks
    pub const UNMOVED_ROOK: u8 = 6;

    pub fn to_board(&self) -> (Board, i16, MarlinWDL) {
        let mut b = Board::empty();
        let mut state = BoardState::default();
        let occ = Bitboard(self.occ);

        let mut castling_squares = ArrayVec::<Square, 4>::new();

        let mut i = 0;
        occ.bitloop(|s| {
            let marlin_pc = self.pcs[i / 2] >> ((i % 2) * 4) & 0xF;

            let c = Color::from(marlin_pc >> 3);
            let p = if marlin_pc & 0b111 == Self::UNMOVED_ROOK {
                castling_squares.push(s);
                Piece::Rook
            } else {
                Piece::from(marlin_pc & 0b111)
            };

            let pc = CPiece::create(c, p);

            state.hash.toggle_piece(pc, s);
            b.set_piece(pc, s);

            i += 1;
        });

        let mut c_rights = CastlingRights::NONE;
        for s in castling_squares {
            let c = b.pc_at(s).color();
            let ksq = b.ksq(c);
            let mask = CastlingRights::get_mask(c, ksq > s);
            b.castlingmask.add_rights(ksq, s, mask);
            c_rights |= mask;
        }

        state.hash.toggle_castling(c_rights);

        let c = Color::from(self.stm_epsq >> 7);
        b.stm = c;
        if c == Color::White {
            state.hash.toggle_color();
        }

        let epsq = Square::from(self.stm_epsq & 0x7F);
        if epsq != Square::Invalid {
            state.epsq = epsq;
            state.hash.toggle_ep(epsq);
        }

        b.state.halfmoves = self.halfmoves as usize;
        b.state.fullmoves = self.fullmoves as usize;

        b.update_masks(&mut state);

        b.state = state;
        (b, i16::from_le(self.eval), self.wdl)
    }

    pub const fn to_bytes(self) -> [u8; size_of::<Self>()] {
        unsafe { std::mem::transmute(self) }
    }

    pub const fn from_bytes(bytes: [u8; size_of::<Self>()]) -> Self {
        unsafe { std::mem::transmute(bytes) }
    }
}
