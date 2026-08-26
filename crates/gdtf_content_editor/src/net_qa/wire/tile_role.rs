//! The graphic roles the Terrain form's picker offers, on the wire.

use gdtf_battle_presenter::TileRole;
use serde::{Deserialize, Serialize};

/// A graphic role a terrain def may be authored with, one arm per offered role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum TileRoleNet {
    /// Default floor tile.
    Floor,
    /// Alternate floor panel.
    FloorAltPanel,
    /// North-south wall.
    Wall,
    /// East-west wall.
    WallEw,
    /// Destructible cover.
    Cover,
    /// Empty emplacement.
    Emplacement,
    /// Intact ceiling slab.
    Slab,
    /// Rubble after destruction.
    Rubble,
    /// Ladder connector.
    Ladder,
    /// North-south door.
    DoorNs,
    /// East-west door.
    DoorEw,
    /// North-south stair up.
    StairNsUp,
    /// North-south stair down.
    StairNsDown,
    /// East-west stair up.
    StairEwUp,
    /// East-west stair down.
    StairEwDown,
}

impl TileRoleNet {
    /// Every role this wire type spells, for a case that walks them all.
    #[cfg(test)]
    pub(in crate::net_qa) const ALL: [Self; 15] = [
        Self::Floor,
        Self::FloorAltPanel,
        Self::Wall,
        Self::WallEw,
        Self::Cover,
        Self::Emplacement,
        Self::Slab,
        Self::Rubble,
        Self::Ladder,
        Self::DoorNs,
        Self::DoorEw,
        Self::StairNsUp,
        Self::StairNsDown,
        Self::StairEwUp,
        Self::StairEwDown,
    ];

    /// Mirror the presenter's own role, or nothing when the form offers no arm for it.
    pub(in crate::net_qa) const fn from_role(role: TileRole) -> Option<Self> {
        match role {
            TileRole::Floor => Some(Self::Floor),
            TileRole::FloorAltPanel => Some(Self::FloorAltPanel),
            TileRole::Wall => Some(Self::Wall),
            TileRole::WallEw => Some(Self::WallEw),
            TileRole::Cover => Some(Self::Cover),
            TileRole::Emplacement => Some(Self::Emplacement),
            TileRole::Slab => Some(Self::Slab),
            TileRole::Rubble => Some(Self::Rubble),
            TileRole::Ladder => Some(Self::Ladder),
            TileRole::DoorNs => Some(Self::DoorNs),
            TileRole::DoorEw => Some(Self::DoorEw),
            TileRole::StairNsUp => Some(Self::StairNsUp),
            TileRole::StairNsDown => Some(Self::StairNsDown),
            TileRole::StairEwUp => Some(Self::StairEwUp),
            TileRole::StairEwDown => Some(Self::StairEwDown),
            TileRole::EmplacementOccupied
            | TileRole::SlabDestroyed
            | TileRole::Door
            | TileRole::StairUp
            | TileRole::StairDown => None,
        }
    }

    /// Read a client's role back as the presenter's own.
    pub(in crate::net_qa) const fn to_role(self) -> TileRole {
        match self {
            Self::Floor => TileRole::Floor,
            Self::FloorAltPanel => TileRole::FloorAltPanel,
            Self::Wall => TileRole::Wall,
            Self::WallEw => TileRole::WallEw,
            Self::Cover => TileRole::Cover,
            Self::Emplacement => TileRole::Emplacement,
            Self::Slab => TileRole::Slab,
            Self::Rubble => TileRole::Rubble,
            Self::Ladder => TileRole::Ladder,
            Self::DoorNs => TileRole::DoorNs,
            Self::DoorEw => TileRole::DoorEw,
            Self::StairNsUp => TileRole::StairNsUp,
            Self::StairNsDown => TileRole::StairNsDown,
            Self::StairEwUp => TileRole::StairEwUp,
            Self::StairEwDown => TileRole::StairEwDown,
        }
    }
}
