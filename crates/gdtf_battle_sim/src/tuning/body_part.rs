//! GTW-657: an authored table whose six weights sum to ZERO is **invalid data**,
//! documented all-zero Torso fallback is defense-in-depth for a case no authored
use std::fmt;

use bevy::prelude::Deref;
use serde::Deserialize;

/// `#[serde(transparent)]` lets it parse a bare RON scalar.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct BodyPartWeight(u16);

impl BodyPartWeight {
                            #[must_use]
    pub const fn new(weight: u16) -> Self {
        Self(weight)
    }
}

/// deserialize boundary — `#[serde(try_from)]` routes the parse through the
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "BodyPartWeightsDef")]
pub struct BodyPartWeights {
        pub head:      BodyPartWeight,
        pub torso:     BodyPartWeight,
        pub left_arm:  BodyPartWeight,
        pub right_arm: BodyPartWeight,
        pub left_leg:  BodyPartWeight,
        pub right_leg: BodyPartWeight,
}

impl Default for BodyPartWeights {
    fn default() -> Self {
        Self {
            head:      BodyPartWeight(6),
            torso:     BodyPartWeight(40),
            left_arm:  BodyPartWeight(12),
            right_arm: BodyPartWeight(12),
            left_leg:  BodyPartWeight(15),
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
            "body_part_weights: all six body-part weights are zero — the §4 hit-location \
             roll needs at least one non-zero weight (GTW-657)",
        )
    }
}

impl std::error::Error for AllZeroBodyPartWeights {}

/// A serde intermediate (`#[serde(try_from = "BodyPartWeightsDef")]` on
#[derive(Deserialize)]
struct BodyPartWeightsDef {
        head:      BodyPartWeight,
        torso:     BodyPartWeight,
        left_arm:  BodyPartWeight,
        right_arm: BodyPartWeight,
        left_leg:  BodyPartWeight,
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
            head:      def.head,
            torso:     def.torso,
            left_arm:  def.left_arm,
            right_arm: def.right_arm,
            left_leg:  def.left_leg,
            right_leg: def.right_leg,
        })
    }
}
