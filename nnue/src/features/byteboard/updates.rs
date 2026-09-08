use chess::types::{
    board::Board,
    moves::{Move, MoveFlag},
    piece::{CPiece, Piece},
    square::Square,
};

use crate::features::{
    byteboard::{
        luts::NON_KNIGHTS_MASK,
        vbmi2::{
            VecT, board_to_rays, closest_occupied, incoming_sliders, incoming_threats, outgoing_threats, perm_for, push_discovery,
            push_focus, ray_fill, set_square, splat_board,
        },
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
    let perm = perm_for(sq);
    let (pcs, bits) = board_to_rays(perm, board);
    let closest = closest_occupied(bits);

    // If the piece is added, then it focuses its own threats and removes discovered ones,
    // vice versa for if the piece is removed.
    let (focus, disco) = if ADD { (&mut tds.adds, &mut tds.subs) } else { (&mut tds.subs, &mut tds.adds) };

    // Kings cannot threaten or be threatened.
    if pc.pt() != Piece::King {
        push_focus::<true>(focus, perm, pcs, outgoing_threats(pc, closest), pc, sq);
        push_focus::<false>(focus, perm, pcs, incoming_threats(bits, closest), pc, sq);
    }

    // Pair each slider aimed at `sq` with the first piece directly behind it.
    let sliders = incoming_sliders(bits, closest);
    let victims = (closest & NON_KNIGHTS_MASK).rotate_right(32);
    let valid = ray_fill(sliders) & ray_fill(victims);

    push_discovery(disco, perm, pcs, sliders & valid, victims & valid);
}

/// The threats toggled by replacing `old` on `sq` with `new`. No discovered attacks.
fn on_replace(tds: &mut ThreatDeltas, board: VecT, old: CPiece, new: CPiece, sq: Square) {
    let perm = perm_for(sq);
    let (pcs, bits) = board_to_rays(perm, board);
    let closest = closest_occupied(bits);
    let incoming = incoming_threats(bits, closest);

    // A captured piece is never a king.
    push_focus::<true>(&mut tds.subs, perm, pcs, outgoing_threats(old, closest), old, sq);
    push_focus::<false>(&mut tds.subs, perm, pcs, incoming, old, sq);

    // Kings cannot threaten or be threatened.
    if new.pt() != Piece::King {
        push_focus::<true>(&mut tds.adds, perm, pcs, outgoing_threats(new, closest), new, sq);
        push_focus::<false>(&mut tds.adds, perm, pcs, incoming, new, sq);
    }
}
