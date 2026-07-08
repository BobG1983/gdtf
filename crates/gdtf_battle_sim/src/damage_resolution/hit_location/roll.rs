//! The weighted hit-location roll implementation — [`roll_body_part`]. See the module
//! docs (`super`) for the §4 weighted-pick rule and the all-zero fallback.

use rand::{Rng, RngExt};

use crate::{armor::BodyPart, tuning::BodyPartWeights};

/// Pick the struck [`BodyPart`] by a **weighted roll** over the six parts, using
/// the passed tuning [`BodyPartWeights`] (resolution.md §4).
///
/// The six parts are walked in [`BodyPart::ALL`] canonical order, each weighted
/// by its field in `weights`. A single draw in `0..total` (total = the sum of all
/// six weights) is mapped onto the parts by cumulative weight: the part whose
/// running cumulative weight first exceeds the draw is the one struck. A part with
/// weight `0` owns an empty slice of the range and so can **never** be picked; if
/// exactly one part is non-zero it owns the whole range and is **always** picked
/// (acceptance criterion 3).
///
/// The draw is taken from the injected `rng` (`&mut impl rand::Rng`) — never a
/// global or thread RNG (acceptance criterion 4) — so the same seed yields the
/// same sequence of parts.
///
/// **Degenerate fallback (acceptance criterion 5):** if *all six* weights are
/// zero the total is zero, there is no proportional answer, and there is nothing
/// to draw — so this returns [`BodyPart::Torso`] (the central mass, the natural
/// default landing spot) deterministically, **without panicking** and without
/// touching the RNG. A real tuning never ships all-zero; this is purely a
/// can't-happen guard kept honest.
#[must_use]
pub fn roll_body_part(weights: &BodyPartWeights, rng: &mut impl Rng) -> BodyPart {
    // Sum the six weights as u32 so six u16 weights cannot overflow the total.
    let per_part: [(BodyPart, u32); 6] = [
        (BodyPart::Head, u32::from(*weights.head)),
        (BodyPart::Torso, u32::from(*weights.torso)),
        (BodyPart::LeftArm, u32::from(*weights.left_arm)),
        (BodyPart::RightArm, u32::from(*weights.right_arm)),
        (BodyPart::LeftLeg, u32::from(*weights.left_leg)),
        (BodyPart::RightLeg, u32::from(*weights.right_leg)),
    ];
    let total: u32 = per_part.iter().map(|&(_, w)| w).sum();

    // All-zero fallback: no proportional answer exists, so pick the central mass
    // deterministically rather than draw from an empty range (documented guard).
    // GTW-657: Load rejects authored all-zero tables, so this branch is defense-in-depth.
    let Some(upper) = total.checked_sub(1) else {
        return BodyPart::Torso;
    };

    // One draw in 0..total, mapped onto the parts by cumulative weight. A
    // zero-weight part adds nothing to the running cumulative, so its half-open
    // slice [cumulative, cumulative) is empty and it is never selected.
    let draw: u32 = rng.random_range(0..=upper);
    let mut cumulative: u32 = 0;
    for (part, weight) in per_part {
        cumulative += weight;
        if draw < cumulative {
            return part;
        }
    }

    // Unreachable in practice: `draw < total` and the cumulative sum reaches
    // `total`, so the loop always returns above. Torso is the same central-mass
    // default the all-zero branch uses — no panic, no unwrap.
    BodyPart::Torso
}
