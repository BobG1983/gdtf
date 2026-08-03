use serde::{Deserialize, Serialize};

use crate::{armor::BodyPart, tuning::SeverityScaling};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Severity {
            None,
        Minor,
        Major,
        Critical,
        Fatal,
}

impl Severity {
                pub const ALL: [Self; 5] = [
        Self::None,
        Self::Minor,
        Self::Major,
        Self::Critical,
        Self::Fatal,
    ];

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

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SeverityRank(u8);

impl SeverityRank {
        #[must_use]
    pub const fn new(rank: u8) -> Self {
        Self(rank)
    }
}

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct PartSeverityMod(f32);

impl PartSeverityMod {
            #[must_use]
    pub const fn new(part_mod: f32) -> Self {
        Self(part_mod)
    }
}

#[must_use]
pub const fn part_severity_mod(part: BodyPart) -> PartSeverityMod {
    match part {
        BodyPart::Head => PartSeverityMod(12.0),
        BodyPart::Torso => PartSeverityMod(5.0),
        BodyPart::LeftArm | BodyPart::RightArm => PartSeverityMod(-2.0),
        BodyPart::LeftLeg | BodyPart::RightLeg => PartSeverityMod(0.0),
    }
}

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct SeverityScore(f32);

impl SeverityScore {
        #[must_use]
    pub const fn new(score: f32) -> Self {
        Self(score)
    }
}

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
