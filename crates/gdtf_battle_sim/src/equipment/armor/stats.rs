use bevy::prelude::{Component, Deref, DerefMut};
use serde::{Deserialize, Serialize};

/// (house style); a magnitude is TBD tuning. `#[serde(transparent)]` lets an
/// `#[serde(transparent)]`.
#[derive(
    Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default,
)]
#[serde(transparent)]
pub struct ArmorFloor(i32);

impl ArmorFloor {
        #[must_use]
    pub const fn new(floor: i32) -> Self {
        Self(floor)
    }
}

/// `#[serde(transparent)]` lets an authored protection parse as a bare integer.
/// bare integer via `#[serde(transparent)]`.
#[derive(
    Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default,
)]
#[serde(transparent)]
pub struct ArmorProtection(i32);

impl ArmorProtection {
        #[must_use]
    pub const fn new(protection: i32) -> Self {
        Self(protection)
    }
}

/// `#[serde(transparent)]` lets an authored integrity parse as a bare integer.
/// `#[serde(transparent)]`.
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
            #[must_use]
    pub const fn new(integrity: i32) -> Self {
        Self(integrity)
    }
}

/// a magnitude is TBD tuning. `#[serde(transparent)]` lets an authored hardness
/// bare integer via `#[serde(transparent)]`.
#[derive(
    Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default,
)]
#[serde(transparent)]
pub struct ArmorHardness(i32);

impl ArmorHardness {
        #[must_use]
    pub const fn new(hardness: i32) -> Self {
        Self(hardness)
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum BodyPart {
        #[default]
    Head,
        Torso,
        LeftArm,
        RightArm,
        LeftLeg,
        RightLeg,
}

impl BodyPart {
            pub const ALL: [Self; 6] = [
        Self::Head,
        Self::Torso,
        Self::LeftArm,
        Self::RightArm,
        Self::LeftLeg,
        Self::RightLeg,
    ];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum InjuryCategory {
        Head,
        Torso,
        Arm,
        Leg,
}

impl InjuryCategory {
            pub const ALL: [Self; 4] = [Self::Head, Self::Torso, Self::Arm, Self::Leg];
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub enum ArmorType {
        #[default]
    Plated,
        Refractive,
        Flak,
        Void,
        Hazard,
        Reinforced,
        Ceramic,
}

impl ArmorType {
                    pub const ALL: [Self; 7] = [
        Self::Plated,
        Self::Refractive,
        Self::Flak,
        Self::Void,
        Self::Hazard,
        Self::Reinforced,
        Self::Ceramic,
    ];

                            pub const DEFAULT: Self = Self::Plated;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub struct ArmorPiece {
        pub floor:      ArmorFloor,
        pub protection: ArmorProtection,
        pub integrity:  ArmorIntegrity,
        pub hardness:   ArmorHardness,
        pub armor_type: ArmorType,
}

impl ArmorPiece {
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
