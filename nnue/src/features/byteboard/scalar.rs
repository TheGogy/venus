use chess::types::{piece::CPiece, square::Square};
use utils::{cfor, memory::Align64};

use crate::features::{
    byteboard::{DISCOVERY_WIDTH, FOCUS_WIDTH, RayT},
    threat::{ThreatDelta, ThreatDeltaList},
};

/// The board laid out along the 64 ray slots around one square.
pub struct Rays {
    pub pcs: Align64<[u8; 64]>,
    pub sqs: &'static [u8; 64],

    /// The nearest piece in each direction.
    pub closest: RayT,
    /// Which of those pieces attack the square.
    pub threats: RayT,
    /// Which of those are sliders aimed at the square.
    pub sliders: RayT,
}

#[expect(clippy::cast_possible_truncation)]
pub static SLOT_IOTA: Align64<[u8; 64]> = {
    let mut iota = [0; 64];
    cfor!(let mut i = 0; i < 64; i += 1; { iota[i] = i as u8; });
    Align64(iota)
};

/// The indices of the set bits of `x`, from the bottom up.
fn slots(mut x: RayT) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        (x != 0).then(|| {
            let i = x.trailing_zeros() as usize;
            x &= x - 1;
            i
        })
    })
}

impl Rays {
    /// The piece in slot `i` and the square it stands on.
    fn slot(&self, i: usize) -> (CPiece, Square) {
        (CPiece::from_raw(self.pcs[i]), Square::from_raw(self.sqs[i]))
    }

    /// Push every threat between `pc` on `sq` and the pieces picked out by `rays`.
    pub fn push_focus<const OUTGOING: bool>(&self, dst: &mut ThreatDeltaList, rays: RayT, pc: CPiece, sq: Square) {
        debug_assert!(rays.count_ones() as usize <= FOCUS_WIDTH);
        for i in slots(rays) {
            let (o_pc, o_sq) = self.slot(i);
            dst.push(if OUTGOING { ThreatDelta::new(pc, sq, o_pc, o_sq) } else { ThreatDelta::new(o_pc, o_sq, pc, sq) });
        }
    }

    /// Push every `sliders[i]` -> `victims[i]` threat.
    pub fn push_discovery(&self, dst: &mut ThreatDeltaList, sliders: RayT, victims: RayT) {
        debug_assert_eq!(sliders.count_ones(), victims.count_ones());
        debug_assert!(sliders.count_ones() as usize <= DISCOVERY_WIDTH);
        for (s, v) in slots(sliders).zip(slots(victims)) {
            let (slider_pc, slider_sq) = self.slot(s);
            let (victim_pc, victim_sq) = self.slot((v + 32) % 64);
            dst.push(ThreatDelta::new(slider_pc, slider_sq, victim_pc, victim_sq));
        }
    }
}
