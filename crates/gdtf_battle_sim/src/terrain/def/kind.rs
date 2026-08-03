//! Sim and presenter kinds for terrain pieces.

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

/// Simulation-facing terrain kind with combat stats.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum TerrainSimKind {
    /// Solid wall.
    Wall {
        /// Hit points.
        hp: CoverHp,
        /// Armor protection.
        armor_protection: ArmorProtection,
        /// Armor hardness.
        armor_hardness: ArmorHardness,
        /// Height band for vision/brace.
        height_band: HeightBand,
    },
    /// Destructible cover.
    Cover {
        /// Hit points.
        hp: CoverHp,
        /// Armor protection.
        armor_protection: ArmorProtection,
        /// Armor hardness.
        armor_hardness: ArmorHardness,
        /// Height band.
        height_band: HeightBand,
    },
    /// Walkable slab / floor.
    Slab {
        /// Hit points.
        hp: SlabHp,
        /// Armor protection.
        armor_protection: ArmorProtection,
        /// Armor hardness.
        armor_hardness: ArmorHardness,
    },
    /// Weapon emplacement.
    Emplacement {
        /// Hit points.
        hp: CoverHp,
        /// Armor protection.
        armor_protection: ArmorProtection,
        /// Armor hardness.
        armor_hardness: ArmorHardness,
        /// Height band.
        height_band: HeightBand,
        /// Mounted weapon key.
        mounted_weapon: WeaponName,
    },
}

impl TerrainSimKind {
    /// Coarse piece kind.
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

/// Presenter-facing terrain kind with graphics.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum TerrainPresenterKind {
    /// Wall graphic.
    Wall {
        /// Graphic key.
        graphic_name: TerrainGraphicKey,
    },
    /// Cover graphic.
    Cover {
        /// Graphic key.
        graphic_name: TerrainGraphicKey,
    },
    /// Slab graphic and optional footfall.
    Slab {
        /// Graphic key.
        graphic_name: TerrainGraphicKey,
        /// Optional footfall sound.
        footfall: Option<FootfallSound>,
    },
    /// Emplacement graphic.
    Emplacement {
        /// Graphic key.
        graphic_name: TerrainGraphicKey,
    },
}

impl TerrainPresenterKind {
    /// Coarse piece kind.
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

/// Optional behavior tags on a terrain def.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum TerrainTag {
    /// Can be opened.
    Openable,
    /// Blocks vision.
    BlocksVision,
    /// Blocks pathfinding.
    BlocksPathfinding,
    /// Cannot be destroyed.
    Indestructible,
}

/// How a piece blocks line of sight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum LosBlocking {
    /// Full occlusion.
    Full,
    /// Occludes up to the piece height band.
    UpToHeightBand,
    /// No occlusion.
    None,
}
