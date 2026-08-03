//! Per-piece armor stats and body-part enum.

use bevy::prelude::{Component, Deref, DerefMut};
use serde::{Deserialize, Serialize};

/// Damage floor for a piece.
#[derive(
    Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default,
)]
#[serde(transparent)]
pub struct ArmorFloor(i32);

impl ArmorFloor {
    /// Wrap a floor value.
    #[must_use]
    pub const fn new(floor: i32) -> Self {
        Self(floor)
    }
}

/// Protection rating.
#[derive(
    Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default,
)]
#[serde(transparent)]
pub struct ArmorProtection(i32);

impl ArmorProtection {
    /// Wrap a protection value.
    #[must_use]
    pub const fn new(protection: i32) -> Self {
        Self(protection)
    }
}

/// Remaining integrity (mutable under wear).
#[derive(
    Deref,
    DerefMut,
    Component,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Deserialize,
    Serialize,
    Default,
)]
#[serde(transparent)]
pub struct ArmorIntegrity(i32);

impl ArmorIntegrity {
    /// Wrap an integrity value.
    #[must_use]
    pub const fn new(integrity: i32) -> Self {
        Self(integrity)
    }
}

/// Hardness rating.
#[derive(
    Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default,
)]
#[serde(transparent)]
pub struct ArmorHardness(i32);

impl ArmorHardness {
    /// Wrap a hardness value.
    #[must_use]
    pub const fn new(hardness: i32) -> Self {
        Self(hardness)
    }
}

/// Hit location on the body.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum BodyPart {
    /// Head.
    #[default]
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

impl BodyPart {
    /// All six parts in index order.
    pub const ALL: [Self; 6] = [
        Self::Head,
        Self::Torso,
        Self::LeftArm,
        Self::RightArm,
        Self::LeftLeg,
        Self::RightLeg,
    ];

    /// Stable index 0..5.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Head => 0,
            Self::Torso => 1,
            Self::LeftArm => 2,
            Self::RightArm => 3,
            Self::LeftLeg => 4,
            Self::RightLeg => 5,
        }
    }

    /// Injury table category for this part.
    #[must_use]
    pub const fn injury_category(self) -> InjuryCategory {
        match self {
            Self::Head => InjuryCategory::Head,
            Self::Torso => InjuryCategory::Torso,
            Self::LeftArm | Self::RightArm => InjuryCategory::Arm,
            Self::LeftLeg | Self::RightLeg => InjuryCategory::Leg,
        }
    }
}

/// Coarse category for injury tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum InjuryCategory {
    /// Head injuries.
    Head,
    /// Torso injuries.
    Torso,
    /// Arm injuries.
    Arm,
    /// Leg injuries.
    Leg,
}

impl InjuryCategory {
    /// All categories.
    pub const ALL: [Self; 4] = [Self::Head, Self::Torso, Self::Arm, Self::Leg];
}

/// Material / construction type for matchup.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub enum ArmorType {
    /// Plated (default).
    #[default]
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

impl ArmorType {
    /// All types.
    pub const ALL: [Self; 7] = [
        Self::Plated,
        Self::Refractive,
        Self::Flak,
        Self::Void,
        Self::Hazard,
        Self::Reinforced,
        Self::Ceramic,
    ];

    /// Default type.
    pub const DEFAULT: Self = Self::Plated;
}

/// One body-part armor piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub struct ArmorPiece {
    /// Damage floor.
    pub floor: ArmorFloor,
    /// Protection.
    pub protection: ArmorProtection,
    /// Integrity.
    pub integrity: ArmorIntegrity,
    /// Hardness.
    pub hardness: ArmorHardness,
    /// Armor type.
    pub armor_type: ArmorType,
}

impl ArmorPiece {
    /// Build a piece.
    #[must_use]
    pub const fn new(
        floor: ArmorFloor,
        protection: ArmorProtection,
        integrity: ArmorIntegrity,
        hardness: ArmorHardness,
        armor_type: ArmorType,
    ) -> Self {
        Self {
            floor,
            protection,
            integrity,
            hardness,
            armor_type,
        }
    }
}
