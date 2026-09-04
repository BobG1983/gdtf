//! Wounds and lasting injuries on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::{
    armor::BodyPart, inflicted_wound::InflictedWound, injuries::GainedInjury, severity::Severity,
};
use serde::{Deserialize, Serialize};

/// How bad a wound or injury is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeverityNet {
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

impl SeverityNet {
    /// Mirror the sim's severity rank.
    #[must_use]
    pub const fn from_sim(tier: Severity) -> Self {
        match tier {
            Severity::None => Self::None,
            Severity::Minor => Self::Minor,
            Severity::Major => Self::Major,
            Severity::Critical => Self::Critical,
            Severity::Fatal => Self::Fatal,
        }
    }
}

/// Which part of the body took the hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyPartNet {
    /// Head.
    Head,
    /// Torso.
    Torso,
    /// Left arm.
    LeftArm,
    /// Right arm.
    RightArm,
    /// Left leg.
    LeftLeg,
    /// Right leg.
    RightLeg,
}

impl BodyPartNet {
    /// Mirror the sim's body part.
    #[must_use]
    pub const fn from_sim(part: BodyPart) -> Self {
        match part {
            BodyPart::Head => Self::Head,
            BodyPart::Torso => Self::Torso,
            BodyPart::LeftArm => Self::LeftArm,
            BodyPart::RightArm => Self::RightArm,
            BodyPart::LeftLeg => Self::LeftLeg,
            BodyPart::RightLeg => Self::RightLeg,
        }
    }
}

/// One wound the stat block lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WoundNet {
    /// How bad it is.
    pub severity: SeverityNet,
    /// Where it landed.
    pub part:     BodyPartNet,
}

impl WoundNet {
    /// Build from severity and body part.
    #[must_use]
    pub const fn new(severity: SeverityNet, part: BodyPartNet) -> Self {
        Self { severity, part }
    }

    /// Mirror a sim wound record.
    #[must_use]
    pub const fn from_sim(wound: InflictedWound) -> Self {
        Self::new(
            SeverityNet::from_sim(wound.tier),
            BodyPartNet::from_sim(wound.location),
        )
    }
}

/// Injury key as the content tables name it.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InjuryNameNet(String);

impl InjuryNameNet {
    /// Build from an injury name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// One lasting injury the stat block lists.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InjuryNet {
    /// Injury key.
    pub name:     InjuryNameNet,
    /// Where it landed.
    pub part:     BodyPartNet,
    /// How bad it is.
    pub severity: SeverityNet,
}

impl InjuryNet {
    /// Build from name, body part and severity.
    #[must_use]
    pub const fn new(name: InjuryNameNet, part: BodyPartNet, severity: SeverityNet) -> Self {
        Self {
            name,
            part,
            severity,
        }
    }

    /// Mirror a sim injury record.
    #[must_use]
    pub fn from_sim(injury: &GainedInjury) -> Self {
        Self::new(
            InjuryNameNet::new((*injury.name).clone()),
            BodyPartNet::from_sim(injury.part),
            SeverityNet::from_sim(injury.severity),
        )
    }
}
