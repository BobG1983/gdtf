//! Severity ranks and body-part modifiers.

use serde::{Deserialize, Serialize};

use crate::{armor::BodyPart, tuning::SeverityScaling};

/// How bad a wound is, from nothing through fatal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Severity {
    /// No meaningful wound.
    None,
    /// Light injury.
    Minor,
    /// Serious injury.
    Major,
    /// Life-threatening injury.
    Critical,
    /// Instantly or near-instantly lethal.
    Fatal,
}

impl Severity {
    /// All ranks in ascending order.
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Minor,
        Self::Major,
        Self::Critical,
        Self::Fatal,
    ];

    /// Numeric rank used for comparisons and costs.
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

/// Numeric rank of a [`Severity`] (0 = none … 4 = fatal).
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SeverityRank(u8);

impl SeverityRank {
    /// Build from a raw rank value.
    #[must_use]
    pub const fn new(rank: u8) -> Self {
        Self(rank)
    }
}

/// Additive modifier applied to the severity score for a body part.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct PartSeverityMod(f32);

impl PartSeverityMod {
    /// Build from a raw modifier.
    #[must_use]
    pub const fn new(part_mod: f32) -> Self {
        Self(part_mod)
    }
}

/// Severity score bias for a given body part (head is harsh, arms are forgiving).
#[must_use]
pub const fn part_severity_mod(part: BodyPart) -> PartSeverityMod {
    match part {
        BodyPart::Head => PartSeverityMod(12.0),
        BodyPart::Torso => PartSeverityMod(5.0),
        BodyPart::LeftArm | BodyPart::RightArm => PartSeverityMod(-2.0),
        BodyPart::LeftLeg | BodyPart::RightLeg => PartSeverityMod(0.0),
    }
}

/// Raw numeric score before it is bucketed into a [`Severity`].
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct SeverityScore(f32);

impl SeverityScore {
    /// Build from a raw score.
    #[must_use]
    pub const fn new(score: f32) -> Self {
        Self(score)
    }
}

/// Map a score onto a severity rank using the configured edges.
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
