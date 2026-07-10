//! The severity vocabulary — the [`Severity`] bucket ladder, the per-part
//! [`PartSeverityMod`] escalation, and the [`bucket`] cutpoint mapping.

use serde::{Deserialize, Serialize};

use crate::{armor::BodyPart, tuning::SeverityScaling};

/// The **severity bucket** a hit's §6 score falls into (resolution.md §6).
///
/// A named domain enum (no bare integer): the score is tiered by the ascending
/// edges `e0..e3` into one of these five outcomes. The ladder ascends from a
/// harmless graze ([`None`](Severity::None)) to death
/// ([`Fatal`](Severity::Fatal)); each non-`None` bucket costs the defender Wounds
/// (the per-tier Wounds cost is a later E3 slice). `Serialize` is added (GTW-654)
/// so the content editor's INJURY authoring mode can write an edited def's
/// `severity:` field back to disk (behavior-inert for the sim).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Severity {
    /// A **graze** (`< e0`): HP loss only, **no Wound** is spent — the
    /// penetration-gated floor of the ladder.
    None,
    /// A **minor** wound (`< e1`): the lightest injury that costs a Wound.
    Minor,
    /// A **major** wound (`< e2`): a worse injury, costing more Wounds.
    Major,
    /// A **critical** wound (`< e3`): a severe injury.
    Critical,
    /// A **fatal** hit (`≥ e3`): empties the Wounds pool — death in battle.
    Fatal,
}

impl Severity {
    /// The five severity buckets in ascending order (graze → death) — the
    /// canonical ordering the bucketing climbs through. Iteration order for
    /// tests and any per-severity lookup.
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Minor,
        Self::Major,
        Self::Critical,
        Self::Fatal,
    ];

    /// This bucket's [`SeverityRank`] on the ascending ladder (`None` = 0 …
    /// `Fatal` = 4) — the order used to compare two severities as relations
    /// (monotone / directional tests), without pinning any score magnitude.
    #[must_use]
    pub const fn rank(self) -> SeverityRank {
        match self {
            Self::None => SeverityRank::new(0),
            Self::Minor => SeverityRank::new(1),
            Self::Major => SeverityRank::new(2),
            Self::Critical => SeverityRank::new(3),
            Self::Fatal => SeverityRank::new(4),
        }
    }
}

/// A [`Severity`]'s **rank** on the ascending ladder — `None = 0 … Fatal = 4`.
///
/// The order used to compare two severities as relations (monotone / directional
/// tests) without pinning any score magnitude. A named domain ordinal (no bare
/// `u8`): private inner + derived [`Deref`](std::ops::Deref); derives [`Ord`] so
/// two ranks compare directly.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SeverityRank(u8);

impl SeverityRank {
    /// Build a severity rank from its ladder position (`0 = None … 4 = Fatal`).
    #[must_use]
    pub const fn new(rank: u8) -> Self {
        Self(rank)
    }
}

/// The **per-part severity modifier** — how much the struck [`BodyPart`] pushes
/// the §6 severity score (resolution.md §6: head +12 / torso +5 / arms −2 /
/// legs 0).
///
/// A named domain newtype (no bare `f32`): a signed score addend (the head
/// amplifies severity, the arms slightly dampen it). This is a **placeholder code
/// const** (resolution.md §6: "the per-part mods currently sit as a placeholder
/// code const"), **distinct** from the §4 hit-location
/// [`crate::tuning::BodyPartWeights`] — that decides *which* part is struck
/// (hit-likelihood), this decides how much a hit *there* escalates severity. An
/// `f32` so it sums directly into the `f32` score. Private inner + derived
/// [`Deref`](std::ops::Deref).
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct PartSeverityMod(f32);

impl PartSeverityMod {
    /// Build a per-part severity modifier from its signed score-addend magnitude
    /// (a placeholder; resolution.md §6).
    #[must_use]
    pub const fn new(part_mod: f32) -> Self {
        Self(part_mod)
    }
}

/// The placeholder per-part severity modifier for the struck [`BodyPart`]
/// (resolution.md §6: head +12 / torso +5 / arms −2 / legs 0).
///
/// A `const` accessor — the head amplifies a hit's severity, the torso a little,
/// the arms slightly dampen it, the legs are neutral. Both arms share the arm
/// value and both legs the leg value (the parts are split L/R, but the severity
/// escalation is the same per limb pair). This is **severity escalation**, NOT
/// the §4 hit-location weight ([`crate::tuning::BodyPartWeights`]). A placeholder
/// until the mods move to tuning data.
#[must_use]
pub const fn part_severity_mod(part: BodyPart) -> PartSeverityMod {
    match part {
        BodyPart::Head => PartSeverityMod(12.0),
        BodyPart::Torso => PartSeverityMod(5.0),
        BodyPart::LeftArm | BodyPart::RightArm => PartSeverityMod(-2.0),
        BodyPart::LeftLeg | BodyPart::RightLeg => PartSeverityMod(0.0),
    }
}

/// The **§6 severity score** — the scalar the wound-severity roll computes and the
/// [`bucket`] cutpoints tier into a [`Severity`] (resolution.md §6).
///
/// A named domain value (no bare `f32`): the score is the weighted sum
/// `j·pen − k·Toughness + part_mod + fatal_bias + I·Luck_shooter + roll`, produced by
/// [`severity_score`](super::roll::severity_score) and consumed only by [`bucket`].
/// Private inner + derived [`Deref`](std::ops::Deref); build one via
/// [`SeverityScore::new`]. The magnitudes are unpinned tuning — tests assert the
/// score's *relations* (monotonicity, directional Luck), never an absolute value.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct SeverityScore(f32);

impl SeverityScore {
    /// Build a severity score from its computed §6 magnitude.
    #[must_use]
    pub const fn new(score: f32) -> Self {
        Self(score)
    }
}

/// Bucket a §6 severity `score` into a [`Severity`] via the ascending edges
/// `e0..e3` (resolution.md §6: `< e0 → None`, `< e1 → Minor`, `< e2 → Major`,
/// `< e3 → Critical`, `≥ e3 → Fatal`).
///
/// Split out so the cutpoints live in one place; relies on the edges climbing
/// (`e0 < e1 < e2 < e3`, the [`crate::tuning::SeverityEdges`] invariant) for the
/// buckets to be monotone in the score.
pub(super) fn bucket(score: SeverityScore, scaling: &SeverityScaling) -> Severity {
    let edges = &scaling.edges;
    if *score < *edges.e0 {
        Severity::None
    } else if *score < *edges.e1 {
        Severity::Minor
    } else if *score < *edges.e2 {
        Severity::Major
    } else if *score < *edges.e3 {
        Severity::Critical
    } else {
        Severity::Fatal
    }
}
