//! Sim and presenter kind enums for terrain defs.

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{
        entity::TerrainPieceKind,
        facing::TerrainFacing,
        piece::{FootfallSound, TerrainGraphicKey},
    },
    weapon::WeaponName,
};

/// Simulation side of a terrain piece.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum TerrainSimKind {
    /// Full wall.
    Wall {
        /// Hit points.
        hp:               CoverHp,
        /// Armor protection.
        armor_protection: ArmorProtection,
        /// Armor hardness.
        armor_hardness:   ArmorHardness,
        /// Height band for cover.
        height_band:      HeightBand,
    },
    /// Partial cover.
    Cover {
        /// Hit points.
        hp:               CoverHp,
        /// Armor protection.
        armor_protection: ArmorProtection,
        /// Armor hardness.
        armor_hardness:   ArmorHardness,
        /// Height band for cover.
        height_band:      HeightBand,
    },
    /// Floor slab.
    Slab {
        /// Hit points.
        hp:               SlabHp,
        /// Armor protection.
        armor_protection: ArmorProtection,
        /// Armor hardness.
        armor_hardness:   ArmorHardness,
    },
    /// Weapon emplacement.
    Emplacement {
        /// Hit points.
        hp:               CoverHp,
        /// Armor protection.
        armor_protection: ArmorProtection,
        /// Armor hardness.
        armor_hardness:   ArmorHardness,
        /// Height band for cover.
        height_band:      HeightBand,
        /// Mounted weapon key.
        mounted_weapon:   WeaponName,
        /// Sides the emplacement can be entered from, in its unrotated frame.
        #[serde(default)]
        entry_sides:      Vec<TerrainFacing>,
    },
}

impl TerrainSimKind {
    /// Runtime piece kind.
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

// Where a cardinal sits in the ring `TerrainFacing::ALL` walks.
fn ring_position(facing: TerrainFacing) -> Option<usize> {
    TerrainFacing::ALL
        .iter()
        .position(|cardinal| *cardinal == facing)
}

/// The authored entry sides read in the frame of a piece turned to `facing`.
///
/// Each side advances by the ring steps from [`TerrainFacing::default`] to `facing`.
#[must_use]
pub fn rotated_entry_sides(
    authored: &[TerrainFacing],
    facing: TerrainFacing,
) -> Vec<TerrainFacing> {
    let ring = TerrainFacing::ALL;
    let (Some(from), Some(to)) = (
        ring_position(TerrainFacing::default()),
        ring_position(facing),
    ) else {
        return authored.to_vec();
    };
    let steps = (to + ring.len() - from) % ring.len();
    authored
        .iter()
        .filter_map(|side| ring_position(*side).map(|at| ring[(at + steps) % ring.len()]))
        .collect()
}

/// Presenter side of a terrain piece.
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
        footfall:     Option<FootfallSound>,
    },
    /// Emplacement graphic.
    Emplacement {
        /// Graphic key.
        graphic_name: TerrainGraphicKey,
    },
}

impl TerrainPresenterKind {
    /// Runtime piece kind.
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

/// Optional behaviour tags on a terrain def.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum TerrainTag {
    /// Can be opened and closed.
    Openable,
    /// A staircase, for the view derivation. Carries no blocking behaviour.
    Stair,
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
    /// Full vision block.
    Full,
    /// Block up to the piece height band.
    UpToHeightBand,
    /// No vision block.
    None,
}
