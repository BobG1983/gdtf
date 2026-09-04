//! The Armor form's body parts, armor types and per-piece stats on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart,
};
use serde::{Deserialize, Serialize};

/// One of the six pieces an armor suit is authored in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum BodyPartNet {
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
    /// Mirror the sim's own body part.
    pub(in crate::mcp) const fn from_part(part: BodyPart) -> Self {
        match part {
            BodyPart::Head => Self::Head,
            BodyPart::Torso => Self::Torso,
            BodyPart::LeftArm => Self::LeftArm,
            BodyPart::RightArm => Self::RightArm,
            BodyPart::LeftLeg => Self::LeftLeg,
            BodyPart::RightLeg => Self::RightLeg,
        }
    }

    /// Read a client's body part back as the sim's own.
    pub(in crate::mcp) const fn to_part(self) -> BodyPart {
        match self {
            Self::Head => BodyPart::Head,
            Self::Torso => BodyPart::Torso,
            Self::LeftArm => BodyPart::LeftArm,
            Self::RightArm => BodyPart::RightArm,
            Self::LeftLeg => BodyPart::LeftLeg,
            Self::RightLeg => BodyPart::RightLeg,
        }
    }
}

/// The material a piece is built from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum ArmorTypeNet {
    /// Plated.
    Plated,
    /// Refractive.
    Refractive,
    /// Flak.
    Flak,
    /// Void-hardened.
    Void,
    /// Hazard suit.
    Hazard,
    /// Reinforced.
    Reinforced,
    /// Ceramic.
    Ceramic,
}

impl ArmorTypeNet {
    /// Mirror the sim's own armor type.
    pub(in crate::mcp) const fn from_type(armor_type: ArmorType) -> Self {
        match armor_type {
            ArmorType::Plated => Self::Plated,
            ArmorType::Refractive => Self::Refractive,
            ArmorType::Flak => Self::Flak,
            ArmorType::Void => Self::Void,
            ArmorType::Hazard => Self::Hazard,
            ArmorType::Reinforced => Self::Reinforced,
            ArmorType::Ceramic => Self::Ceramic,
        }
    }

    /// Read a client's armor type back as the sim's own.
    pub(in crate::mcp) const fn to_type(self) -> ArmorType {
        match self {
            Self::Plated => ArmorType::Plated,
            Self::Refractive => ArmorType::Refractive,
            Self::Flak => ArmorType::Flak,
            Self::Void => ArmorType::Void,
            Self::Hazard => ArmorType::Hazard,
            Self::Reinforced => ArmorType::Reinforced,
            Self::Ceramic => ArmorType::Ceramic,
        }
    }
}

/// A piece's damage floor.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct ArmorFloorNet(i32);

impl ArmorFloorNet {
    /// Mirror the sim's own floor.
    pub(in crate::mcp) fn from_floor(floor: ArmorFloor) -> Self {
        Self(*floor)
    }

    /// Read a client's floor back as the sim's own.
    pub(in crate::mcp) const fn to_floor(self) -> ArmorFloor {
        ArmorFloor::new(self.0)
    }
}

/// A piece's protection rating.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct ArmorProtectionNet(i32);

impl ArmorProtectionNet {
    /// Mirror the sim's own protection.
    pub(in crate::mcp) fn from_protection(protection: ArmorProtection) -> Self {
        Self(*protection)
    }

    /// Read a client's protection back as the sim's own.
    pub(in crate::mcp) const fn to_protection(self) -> ArmorProtection {
        ArmorProtection::new(self.0)
    }
}

/// A piece's hardness rating.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct ArmorHardnessNet(i32);

impl ArmorHardnessNet {
    /// Mirror the sim's own hardness.
    pub(in crate::mcp) fn from_hardness(hardness: ArmorHardness) -> Self {
        Self(*hardness)
    }

    /// Read a client's hardness back as the sim's own.
    pub(in crate::mcp) const fn to_hardness(self) -> ArmorHardness {
        ArmorHardness::new(self.0)
    }
}

/// A piece's remaining integrity.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct ArmorIntegrityNet(i32);

impl ArmorIntegrityNet {
    /// Mirror the sim's own integrity.
    pub(in crate::mcp) fn from_integrity(integrity: ArmorIntegrity) -> Self {
        Self(*integrity)
    }

    /// Read a client's integrity back as the sim's own.
    pub(in crate::mcp) const fn to_integrity(self) -> ArmorIntegrity {
        ArmorIntegrity::new(self.0)
    }
}
