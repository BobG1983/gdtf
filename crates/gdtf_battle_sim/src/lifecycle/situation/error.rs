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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleSetupError {
            InvalidLink(InvalidVerticalLink),
                        GangNotFound {
                gang: GangName,
    },
                        GangMemberNotFound {
                gang:   GangName,
                member: GangerName,
    },
                WeaponNotFound {
                weapon: WeaponName,
    },
                    ArmorNotFound {
                armor: ArmorName,
    },
                                        MeleeWeaponNotFound {
                weapon: WeaponName,
    },
                                    TerrainNotFound {
                piece: TerrainUuid,
    },
                                            FloorCostBelowMinimum {
                piece:   TerrainUuid,
                cost:    MoveCost,
                minimum: MoveCost,
    },
                                                            StackedGangers {
                        at: CellLevel,
    },
                                FieldNotFound {
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
