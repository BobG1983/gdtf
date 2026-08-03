//! Hit-location weights for body parts.

use std::fmt;

use bevy::prelude::Deref;
use serde::Deserialize;

/// Relative weight for one body part.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct BodyPartWeight(u16);

impl BodyPartWeight {
    /// Wrap a weight.
    #[must_use]
    pub const fn new(weight: u16) -> Self {
        Self(weight)
    }
}

/// Six-part weight table; rejects all-zero on parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "BodyPartWeightsDef")]
pub struct BodyPartWeights {
    /// Head.
    pub head: BodyPartWeight,
    /// Torso.
    pub torso: BodyPartWeight,
    /// Left arm.
    pub left_arm: BodyPartWeight,
    /// Right arm.
    pub right_arm: BodyPartWeight,
    /// Left leg.
    pub left_leg: BodyPartWeight,
    /// Right leg.
    pub right_leg: BodyPartWeight,
}

impl Default for BodyPartWeights {
    fn default() -> Self {
        Self {
            head: BodyPartWeight(6),
            torso: BodyPartWeight(40),
            left_arm: BodyPartWeight(12),
            right_arm: BodyPartWeight(12),
            left_leg: BodyPartWeight(15),
            right_leg: BodyPartWeight(15),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AllZeroBodyPartWeights;

impl fmt::Display for AllZeroBodyPartWeights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "body_part_weights: all six body-part weights are zero — the hit-location \
             roll needs at least one non-zero weight",
        )
    }
}

impl std::error::Error for AllZeroBodyPartWeights {}

#[derive(Deserialize)]
struct BodyPartWeightsDef {
    head: BodyPartWeight,
    torso: BodyPartWeight,
    left_arm: BodyPartWeight,
    right_arm: BodyPartWeight,
    left_leg: BodyPartWeight,
    right_leg: BodyPartWeight,
}

impl TryFrom<BodyPartWeightsDef> for BodyPartWeights {
    type Error = AllZeroBodyPartWeights;

    fn try_from(def: BodyPartWeightsDef) -> Result<Self, Self::Error> {
        let total = u32::from(*def.head)
            + u32::from(*def.torso)
            + u32::from(*def.left_arm)
            + u32::from(*def.right_arm)
            + u32::from(*def.left_leg)
            + u32::from(*def.right_leg);
        if total == 0 {
            return Err(AllZeroBodyPartWeights);
        }
        Ok(Self {
            head: def.head,
            torso: def.torso,
            left_arm: def.left_arm,
            right_arm: def.right_arm,
            left_leg: def.left_leg,
            right_leg: def.right_leg,
        })
    }
}
