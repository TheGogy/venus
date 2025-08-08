// Just skip the whole file rustfmt, it's more readable this way.
#![cfg_attr(rustfmt, rustfmt_skip)]

// Constants and helpers for bit manipulation.
use chess::types::moves::Move;

use crate::tt::entry::Bound;

// Entry data bit positions
const SHIFT_AGE:   u64 = 1;
const SHIFT_DEPTH: u64 = 7;
const SHIFT_BOUND: u64 = 14;
const SHIFT_MOVE:  u64 = 16;
const SHIFT_EVAL:  u64 = 32;
const SHIFT_VALUE: u64 = 48;

// Entry data masks.
pub const MASK_AGE: u64 = 0x3F;

const MASK_PV:    u64 = 0x1;
const MASK_DEPTH: u64 = 0x7F << SHIFT_DEPTH;
const MASK_BOUND: u64 = 0x3 << SHIFT_BOUND;
const MASK_MOVE:  u64 = 0xFFFF << SHIFT_MOVE;
const MASK_EVAL:  u64 = 0xFFFF << SHIFT_EVAL;
const MASK_VALUE: u64 = 0xFFFF << SHIFT_VALUE;

// Helper functions for packing data.
pub const fn pack_pv(pv: bool)        -> u64 {  pv as u64                           }
pub const fn pack_age(age: u8)        -> u64 { (age as u64)          << SHIFT_AGE   }
pub const fn pack_bound(bound: Bound) -> u64 { (bound as u64)        << SHIFT_BOUND }
pub const fn pack_eval(eval: i16)     -> u64 { (eval as u16 as u64)  << SHIFT_EVAL  }
pub const fn pack_value(value: i16)   -> u64 { (value as u16 as u64) << SHIFT_VALUE }
pub const fn pack_depth(depth: u8)    -> u64 { (depth as u64)        << SHIFT_DEPTH }
pub const fn pack_move(mv: Move)      -> u64 { (mv.0 as u64)         << SHIFT_MOVE  }

// Helper functions for unpacking data.
pub const fn unpack_pv(data: u64)     -> bool  {   data & MASK_PV != 0 }
pub const fn unpack_age(data: u64)    -> u8    { ((data & MASK_AGE) >> SHIFT_AGE) as u8 }
pub const fn unpack_bound(data: u64)  -> Bound { unsafe { std::mem::transmute(((data & MASK_BOUND) >> SHIFT_BOUND) as u8) } }
pub const fn unpack_eval(data: u64)   -> i16   { ((data & MASK_EVAL) >> SHIFT_EVAL) as i16 }
pub const fn unpack_value(data: u64)  -> i16   { ((data & MASK_VALUE) >> SHIFT_VALUE) as i16 }
pub const fn unpack_depth(data: u64)  -> u8    { ((data & MASK_DEPTH) >> SHIFT_DEPTH) as u8 }
pub const fn unpack_move(data: u64)   -> Move  { Move(((data & MASK_MOVE) >> SHIFT_MOVE) as u16) }
