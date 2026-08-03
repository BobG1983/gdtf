//! Errors raised while turning a situation into a live battle.

use crate::{
    armor::ArmorName,
    effects::fields::FieldKey,
    ganger::{GangName, GangerName},
    metric::CellLevel,
    terrain::def::TerrainUuid,
    tuning::MoveCost,
    vertical::InvalidVerticalLink,
    weapon::WeaponName,
};

/// Why battle setup failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleSetupError {
    /// Bad vertical link.
    InvalidLink(InvalidVerticalLink),
    /// Gang missing from catalog.
    GangNotFound {
        /// Requested gang.
        gang: GangName,
    },
    /// Member missing from gang roster.
    GangMemberNotFound {
        /// Gang that was found.
        gang: GangName,
        /// Member that was not.
        member: GangerName,
    },
    /// Weapon missing from catalog.
    WeaponNotFound {
        /// Requested weapon.
        weapon: WeaponName,
    },
    /// Armor missing from catalog.
    ArmorNotFound {
        /// Requested armor.
        armor: ArmorName,
    },
    /// Melee weapon missing from catalog.
    MeleeWeaponNotFound {
        /// Requested melee weapon.
        weapon: WeaponName,
    },
    /// Terrain piece missing from catalog.
    TerrainNotFound {
        /// Requested piece.
        piece: TerrainUuid,
    },
    /// Floor move cost below the minimum.
    FloorCostBelowMinimum {
        /// Floor piece.
        piece: TerrainUuid,
        /// Authored cost.
        cost: MoveCost,
        /// Required minimum.
        minimum: MoveCost,
    },
    /// Two gangers authored on the same cell.
    StackedGangers {
        /// Shared cell.
        at: CellLevel,
    },
    /// Field missing from catalog.
    FieldNotFound {
        /// Requested field.
        field: FieldKey,
    },
}

impl From<InvalidVerticalLink> for BattleSetupError {
    fn from(invalid: InvalidVerticalLink) -> Self {
        Self::InvalidLink(invalid)
    }
}

impl std::fmt::Display for BattleSetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLink(invalid) => write!(f, "invalid vertical link: {invalid:?}"),
            Self::GangNotFound { gang } => {
                write!(f, "no gang `{}` is loaded (missing gang file)", **gang)
            }
            Self::GangMemberNotFound { gang, member } => {
                write!(f, "gang `{}` has no member `{}`", **gang, **member)
            }
            Self::WeaponNotFound { weapon } => {
                write!(f, "no weapon `{}` is loaded", **weapon)
            }
            Self::ArmorNotFound { armor } => write!(f, "no armor `{}` is loaded", **armor),
            Self::MeleeWeaponNotFound { weapon } => {
                write!(f, "no melee weapon `{}` is loaded", **weapon)
            }
            Self::TerrainNotFound { piece } => {
                write!(f, "no terrain piece `{}` is loaded", **piece)
            }
            Self::FloorCostBelowMinimum {
                piece,
                cost,
                minimum,
            } => write!(
                f,
                "floor piece `{}` move cost {} is below the minimum {}",
                **piece, **cost, **minimum
            ),
            Self::StackedGangers { at } => write!(
                f,
                "two or more gangers are authored on the same spawn cell {:?} \
                 (each ganger needs a distinct (cell, level))",
                **at
            ),
            Self::FieldNotFound { field } => {
                write!(f, "no area-damage field `{}` is loaded", **field)
            }
        }
    }
}
