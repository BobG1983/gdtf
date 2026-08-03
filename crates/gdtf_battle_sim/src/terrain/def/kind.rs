use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{
        entity::TerrainPieceKind,
        piece::{FootfallSound, TerrainGraphicKey},
    },
    weapon::WeaponName,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum TerrainSimKind {
                Wall {
                hp:               CoverHp,
                armor_protection: ArmorProtection,
                armor_hardness:   ArmorHardness,
                height_band:      HeightBand,
    },
                Cover {
                hp:               CoverHp,
                armor_protection: ArmorProtection,
                armor_hardness:   ArmorHardness,
                height_band:      HeightBand,
    },
                Slab {
                        hp:               SlabHp,
                armor_protection: ArmorProtection,
                armor_hardness:   ArmorHardness,
    },
                                                            Emplacement {
                        hp:               CoverHp,
                armor_protection: ArmorProtection,
                armor_hardness:   ArmorHardness,
                                height_band:      HeightBand,
                                        mounted_weapon:   WeaponName,
    },
}

impl TerrainSimKind {
                                        #[must_use]
    pub const fn kind(&self) -> TerrainPieceKind {
        match self {
            Self::Wall { .. } => TerrainPieceKind::Wall,
            Self::Cover { .. } => TerrainPieceKind::Cover,
            Self::Slab { .. } => TerrainPieceKind::Slab,
            Self::Emplacement { .. } => TerrainPieceKind::Emplacement,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum TerrainPresenterKind {
        Wall {
                        graphic_name: TerrainGraphicKey,
    },
        Cover {
                        graphic_name: TerrainGraphicKey,
    },
            Slab {
                        graphic_name: TerrainGraphicKey,
                                footfall:     Option<FootfallSound>,
    },
                Emplacement {
                        graphic_name: TerrainGraphicKey,
    },
}

impl TerrainPresenterKind {
                                #[must_use]
    pub const fn kind(&self) -> TerrainPieceKind {
        match self {
            Self::Wall { .. } => TerrainPieceKind::Wall,
            Self::Cover { .. } => TerrainPieceKind::Cover,
            Self::Slab { .. } => TerrainPieceKind::Slab,
            Self::Emplacement { .. } => TerrainPieceKind::Emplacement,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum TerrainTag {
            Openable,
            BlocksVision,
            BlocksPathfinding,
            Indestructible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum LosBlocking {
            Full,
                UpToHeightBand,
        None,
}
