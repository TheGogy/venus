use chess::types::{
    board::Board,
    moves::{Move, MoveFlag},
    piece::{CPiece, Piece},
    square::Square,
};

use crate::features::{
    byteboard::{
        backend::{Rays, VecT, set_square, splat_board},
        luts::NON_KNIGHTS_MASK,
        outgoing_threats, ray_fill,
    },
    threat::ThreatDeltas,
};

/// Collect every threat toggled by `m`. `b` must be the board BEFORE the move.
pub fn on_move(tds: &mut ThreatDeltas, b: &Board, m: Move) {
    let pc = b.pc_at(m.src());
    let (mc, src, dst, flag) = (pc.color(), m.src(), m.dst(), m.flag());
    let new_pc = if flag.is_promo() { CPiece::make(mc, flag.get_promo()) } else { pc };

    let clear = |brd: VecT, sq| set_square(brd, sq, CPiece::None);

    let board = splat_board(b);

    match flag {
        // En passant captures a piece that isn't on `dst` - have to handle separately.
        MoveFlag::EnPassant => {
            let epsq = dst.forward(!mc);
            let victim = CPiece::make(!mc, Piece::Pawn);

            on_change::<false>(tds, board, victim, epsq);
            let board = clear(board, epsq);
            on_change::<false>(tds, board, pc, src);
            on_change::<true>(tds, clear(board, src), new_pc, dst);
        }

        // Castling moves the rook as well as the king.
        MoveFlag::Castling => {
            let (rf, rt) = b.castlingmask.rook_src_dst(dst);
            let rook = CPiece::make(mc, Piece::Rook);

            on_change::<false>(tds, board, pc, src);
            let board = clear(board, src);
            on_change::<false>(tds, board, rook, rf);
            let board = clear(board, rf);
            on_change::<true>(tds, board, pc, dst);
            on_change::<true>(tds, set_square(board, dst, pc), rook, rt);
        }

        // All other captures / capture promos take the piece on `dst` and move
        // the piece on `src`.
        f if f.is_cap() => {
            on_change::<false>(tds, board, pc, src);
            on_replace(tds, clear(board, src), b.pc_at(dst), new_pc, dst);
        }

        // Normal / double / regular promo moves just add and subtract one piece.
        _ => {
            on_change::<false>(tds, board, pc, src);
            on_change::<true>(tds, clear(board, src), new_pc, dst);
        }
    }
}

/// The threats toggled by putting `pc` on `sq` or taking it off again.
fn on_change<const ADD: bool>(tds: &mut ThreatDeltas, board: VecT, pc: CPiece, sq: Square) {
    let rays = Rays::new(board, sq);

    // If the piece is added, then it focuses its own threats and removes discovered ones,
    // vice versa for if the piece is removed.
    let (focus, disco) = if ADD { (&mut tds.adds, &mut tds.subs) } else { (&mut tds.subs, &mut tds.adds) };

    // Kings cannot threaten or be threatened.
    if pc.pt() != Piece::King {
        rays.push_focus::<true>(focus, outgoing_threats(pc, rays.closest), pc, sq);
        rays.push_focus::<false>(focus, rays.threats, pc, sq);
    }

    // Pair each slider aimed at `sq` with the first piece directly behind it.
    let victims = (rays.closest & NON_KNIGHTS_MASK).rotate_right(32);
    let valid = ray_fill(rays.sliders) & ray_fill(victims);

    rays.push_discovery(disco, rays.sliders & valid, victims & valid);
}

/// The threats toggled by replacing `old` on `sq` with `new`. No discovered attacks.
fn on_replace(tds: &mut ThreatDeltas, board: VecT, old: CPiece, new: CPiece, sq: Square) {
    let rays = Rays::new(board, sq);

    // A captured piece is never a king.
    rays.push_focus::<true>(&mut tds.subs, outgoing_threats(old, rays.closest), old, sq);
    rays.push_focus::<false>(&mut tds.subs, rays.threats, old, sq);

    // Kings cannot threaten or be threatened.
    if new.pt() != Piece::King {
        rays.push_focus::<true>(&mut tds.adds, outgoing_threats(new, rays.closest), new, sq);
        rays.push_focus::<false>(&mut tds.adds, rays.threats, new, sq);
    }
}
