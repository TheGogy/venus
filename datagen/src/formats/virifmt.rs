use std::io::{BufRead, Write};

use chess::types::{
    board::Board,
    color::Color,
    eval::Eval,
    moves::{Move, MoveFlag},
};

use crate::formats::marlinfmt::{MarlinFmt, MarlinWDL};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ViriMove(u16);

impl ViriMove {
    fn from_move(m: Move, b: &Board) -> Self {
        let flag = match m.flag() {
            MoveFlag::Normal | MoveFlag::Capture | MoveFlag::DoublePush => 0b0000,

            MoveFlag::EnPassant => 0b0001,
            MoveFlag::Castling => 0b0010,

            MoveFlag::PromoN | MoveFlag::CPromoN => 0b1100,
            MoveFlag::PromoB | MoveFlag::CPromoB => 0b1101,
            MoveFlag::PromoR | MoveFlag::CPromoR => 0b1110,
            MoveFlag::PromoQ | MoveFlag::CPromoQ => 0b1111,
        };

        // We use king src -> dst, virifmt specifies that the king takes the rook
        let dst = if m.flag() == MoveFlag::Castling {
            let (rf, _) = b.castlingmask.rook_src_dst(m.dst());
            rf
        } else {
            m.dst()
        };

        Self((m.src() as u16) | ((dst as u16) << 6) | (flag << 14))
    }
}

#[derive(Debug, Default)]
pub struct ViriFmt {
    pub startpos: MarlinFmt,
    pub moves: Vec<(ViriMove, i16)>,
}

impl ViriFmt {
    pub fn new(b: &Board) -> Self {
        Self { startpos: b.into(), moves: vec![] }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn push(&mut self, b: &Board, m: Move, e: Eval) {
        // Change to white relative eval.
        let s = if b.stm == Color::White { e } else { -e };
        self.moves.push((ViriMove::from_move(m, b), s.0 as i16));
    }

    pub const fn finish(&mut self, wdl: MarlinWDL) {
        self.startpos.wdl = wdl;
    }

    pub fn write(&self, writer: &mut impl Write) -> std::io::Result<()> {
        writer.write_all(&self.startpos.to_bytes())?;
        for (m, e) in &self.moves {
            writer.write_all(&m.0.to_le_bytes())?;
            writer.write_all(&e.to_le_bytes())?;
        }
        writer.write_all(&[0, 0, 0, 0])?;
        Ok(())
    }

    pub fn read_next(&mut self, reader: &mut impl BufRead) -> std::io::Result<()> {
        // Position
        let mut buf = [0; size_of::<MarlinFmt>()];
        reader.read_exact(&mut buf)?;
        self.startpos = MarlinFmt::from_bytes(buf);

        // Moves
        self.moves.clear();
        loop {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            if buf == [0, 0, 0, 0] {
                break;
            }

            let (mv, sc): (ViriMove, i16) = unsafe { std::mem::transmute(buf) };
            self.moves.push((mv, sc));
        }

        Ok(())
    }
}
