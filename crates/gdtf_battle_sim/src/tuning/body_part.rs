//! The §4 body-part hit-location weights (resolution.md §4 `roll_body_part`).

use bevy::prelude::Deref;
use serde::Deserialize;

/// The relative weight of one body part in the §4 `roll_body_part` weighted roll.
///
/// One newtype reused by all six fields of [`BodyPartWeights`]: each part's
/// weight is the same *kind* of value (a relative pick weight), distinguished by
/// its field. A small non-negative integer summed into a weighted pick;
/// `#[serde(transparent)]` lets it parse a bare RON scalar.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct BodyPartWeight(u16);

impl BodyPartWeight {
    /// Build a body-part pick weight from its relative magnitude (TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u16` private (house
    /// style) while letting callers (e.g. the `roll_body_part` roll's tests, or
    /// any code assembling a [`BodyPartWeights`] outside this module) build a
    /// weight without a bare `u16` escaping.
    #[must_use]
    pub const fn new(weight: u16) -> Self {
        Self(weight)
    }
}

/// The body-part hit-location weights — the relative weight of each of the six
/// parts in the §4 `roll_body_part` weighted roll.
///
/// Head is rare, torso the bulk (resolution.md §4: Head 6 / Torso 40 / each Arm
/// 12 / each Leg 15). Each field is a [`BodyPartWeight`]; the magnitudes are
/// tunable balance data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BodyPartWeights {
    /// Head weight — rare (doc default 6).
    pub head:      BodyPartWeight,
    /// Torso weight — the bulk of hits (doc default 40).
    pub torso:     BodyPartWeight,
    /// Left-arm weight (doc default 12).
    pub left_arm:  BodyPartWeight,
    /// Right-arm weight (doc default 12).
    pub right_arm: BodyPartWeight,
    /// Left-leg weight (doc default 15).
    pub left_leg:  BodyPartWeight,
    /// Right-leg weight (doc default 15).
    pub right_leg: BodyPartWeight,
}

impl Default for BodyPartWeights {
    fn default() -> Self {
        // Defaults from docs/combat/resolution.md §4.
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
