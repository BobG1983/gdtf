//! Weighted random body-part selection.

use rand::{Rng, RngExt};

use crate::{armor::BodyPart, tuning::BodyPartWeights};

/// Pick a body part using the configured weights.
/// Falls back to torso if all weights are zero.
#[must_use]
pub fn roll_body_part(weights: &BodyPartWeights, rng: &mut impl Rng) -> BodyPart {
    let per_part: [(BodyPart, u32); 6] = [
        (BodyPart::Head, u32::from(*weights.head)),
        (BodyPart::Torso, u32::from(*weights.torso)),
        (BodyPart::LeftArm, u32::from(*weights.left_arm)),
        (BodyPart::RightArm, u32::from(*weights.right_arm)),
        (BodyPart::LeftLeg, u32::from(*weights.left_leg)),
        (BodyPart::RightLeg, u32::from(*weights.right_leg)),
    ];
    let total: u32 = per_part.iter().map(|&(_, w)| w).sum();

    let Some(upper) = total.checked_sub(1) else {
        return BodyPart::Torso;
    };

    let draw: u32 = rng.random_range(0..=upper);
    let mut cumulative: u32 = 0;
    for (part, weight) in per_part {
        cumulative += weight;
        if draw < cumulative {
            return part;
        }
    }

    BodyPart::Torso
}
