//! The §4 body-part hit-location weights (resolution.md §4 `roll_body_part`).
//!
//! GTW-657: an authored table whose six weights sum to ZERO is **invalid data**,
//! rejected at the deserialize boundary (see [`BodyPartWeights`]) — so the roll's
//! documented all-zero Torso fallback is defense-in-depth for a case no authored
//! tuning can reach.

use std::fmt;

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
///
/// **Load validation (GTW-657):** an authored table whose six weights sum to
/// zero gives the weighted roll no proportional answer, so it is rejected at the
/// deserialize boundary — `#[serde(try_from)]` routes the parse through the
/// `AllZeroBodyPartWeights` check (private; the `GridSize` serde-intermediate shape,
/// `level/theme.rs`; the GTW-643 precedent of a serde-level rejection riding the
/// artifact's existing loud channel). A rejected table fails the whole
/// `RonAsset<CombatTuning>` load, and the hot-RON resolve warns naming
/// `core_tuning/combat.tuning.ron` and falls back to the default — determinism
/// is protected at the data boundary, and the roll's zero-draw Torso fallback
/// stays purely defense-in-depth. Individual zero weights remain legal (a
/// zero-weight part is simply never picked), and in-code struct-literal
/// construction is untouched — the check guards AUTHORED data only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "BodyPartWeightsDef")]
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

/// The GTW-657 load-rejection of an authored all-zero `body_part_weights` table.
///
/// Raised by the serde `try_from` boundary when all six authored weights sum to
/// zero — the degenerate table that would leave the §4 weighted roll nothing to
/// draw from. Its [`Display`](fmt::Display) message names the offending table so
/// the loud load-failure path (the RON loader error + the hot-RON fallback warn
/// naming the file) yields an actionable finding.
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

/// The authored RON shape a [`BodyPartWeights`] deserializes from — the same six
/// per-part fields, routed through the GTW-657 all-zero validation on read.
///
/// A serde intermediate (`#[serde(try_from = "BodyPartWeightsDef")]` on
/// [`BodyPartWeights`]) so the authored table keeps its exact field-per-part
/// shape while the fallible conversion rejects a zero-sum table (the `GridSize`
/// / `GridSizeDef` precedent, `level/theme.rs`). Read-side only: the tuning
/// store has no serialize path, and in-code construction bypasses serde by
/// design (the roll's defense-in-depth fallback stays unit-testable).
#[derive(Deserialize)]
struct BodyPartWeightsDef {
    /// Head weight, as authored.
    head:      BodyPartWeight,
    /// Torso weight, as authored.
    torso:     BodyPartWeight,
    /// Left-arm weight, as authored.
    left_arm:  BodyPartWeight,
    /// Right-arm weight, as authored.
    right_arm: BodyPartWeight,
    /// Left-leg weight, as authored.
    left_leg:  BodyPartWeight,
    /// Right-leg weight, as authored.
    right_leg: BodyPartWeight,
}

impl TryFrom<BodyPartWeightsDef> for BodyPartWeights {
    type Error = AllZeroBodyPartWeights;

    /// Accept the authored table unless its six weights sum to zero (GTW-657).
    fn try_from(def: BodyPartWeightsDef) -> Result<Self, Self::Error> {
        // Sum as u32 so six u16 weights cannot overflow (mirrors the roll's sum).
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
