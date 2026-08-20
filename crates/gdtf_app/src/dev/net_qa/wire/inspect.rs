//! What the inspect panel shows for one cell, on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::{
    cover::{CoverEntry, HeightBand},
    emplacement::EmplacementState,
    entity::TerrainPieceKind,
};
use serde::{Deserialize, Serialize};

use super::roster::GangerCardNet;

/// How much damage cover shrugs off.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HardnessNet(i32);

impl HardnessNet {
    /// Build from a raw hardness value.
    #[must_use]
    pub const fn new(hardness: i32) -> Self {
        Self(hardness)
    }
}

/// How much damage cover soaks for whoever hides behind it.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtectionNet(i32);

impl ProtectionNet {
    /// Build from a raw protection value.
    #[must_use]
    pub const fn new(protection: i32) -> Self {
        Self(protection)
    }
}

/// How tall the cover stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeightBandNet {
    /// Ankle to knee.
    Low,
    /// Knee to chest.
    Mid,
    /// Chest and above.
    High,
}

impl HeightBandNet {
    /// Mirror the sim's height band.
    #[must_use]
    pub const fn from_sim(band: HeightBand) -> Self {
        match band {
            HeightBand::Low => Self::Low,
            HeightBand::Mid => Self::Mid,
            HeightBand::High => Self::High,
        }
    }
}

/// Structural hit points of a cover cell.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CoverHpNet(u32);

impl CoverHpNet {
    /// Build from a raw cover HP value.
    #[must_use]
    pub const fn new(hp: u32) -> Self {
        Self(hp)
    }
}

/// The cover block the inspect panel draws for a terrain cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverBlockNet {
    /// Hardness value.
    pub hardness:   HardnessNet,
    /// Protection value.
    pub protection: ProtectionNet,
    /// Height band.
    pub height:     HeightBandNet,
    /// Current structural HP.
    pub hp:         CoverHpNet,
    /// Structural HP when intact.
    pub hp_max:     CoverHpNet,
}

impl CoverBlockNet {
    /// Mirror a sim cover entry.
    #[must_use]
    pub fn from_sim(entry: CoverEntry) -> Self {
        Self {
            hardness:   HardnessNet::new(*entry.armor_hardness),
            protection: ProtectionNet::new(*entry.armor_protection),
            height:     HeightBandNet::from_sim(entry.height_band),
            hp:         CoverHpNet::new(*entry.current_hp),
            hp_max:     CoverHpNet::new(*entry.max_hp),
        }
    }
}

/// Which kind of terrain piece stands on an inspected cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerrainKindNet {
    /// Full wall.
    Wall,
    /// Partial cover.
    Cover,
    /// Floor slab.
    Slab,
    /// Weapon emplacement.
    Emplacement,
}

impl TerrainKindNet {
    /// Mirror the sim's own piece kind.
    #[must_use]
    pub const fn from_sim(kind: TerrainPieceKind) -> Self {
        match kind {
            TerrainPieceKind::Wall => Self::Wall,
            TerrainPieceKind::Cover => Self::Cover,
            TerrainPieceKind::Slab => Self::Slab,
            TerrainPieceKind::Emplacement => Self::Emplacement,
        }
    }
}

/// Whether an emplacement is vacant or manned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmplacementStateNet {
    /// Nobody is manning it.
    Vacant,
    /// A ganger is manning it.
    Occupied,
}

impl EmplacementStateNet {
    /// Mirror the sim's own emplacement state.
    #[must_use]
    pub const fn from_sim(state: EmplacementState) -> Self {
        match state {
            EmplacementState::Vacant => Self::Vacant,
            EmplacementState::Occupied => Self::Occupied,
        }
    }
}

/// The name of the weapon an emplacement mounts.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MountedWeaponNet(String);

impl MountedWeaponNet {
    /// Wrap a mounted weapon's name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// The terrain half of an inspected cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectTerrainNet {
    /// Which kind of piece stands on the cell.
    pub kind:   TerrainKindNet,
    /// The cover block, when the ledger holds an entry for the cell.
    pub cover:  Option<CoverBlockNet>,
    /// Whether the emplacement is manned, when the cell holds one.
    pub state:  Option<EmplacementStateNet>,
    /// The weapon the emplacement mounts, when the cell holds one.
    pub weapon: Option<MountedWeaponNet>,
}

/// What the inspect panel shows for one cell: the ganger on it and the terrain under it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectShownNet {
    /// The card for a squad-visible ganger standing on the cell.
    pub ganger:  Option<GangerCardNet>,
    /// What the cell's terrain reports, absent when the fog hides the cell.
    pub terrain: Option<InspectTerrainNet>,
}
