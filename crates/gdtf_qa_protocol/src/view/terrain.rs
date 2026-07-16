//! [`TerrainSummaryView`] — the grid summary + the door / emplacement token handout
//! (GTW-734).
//!
//! The wire mirror of the interactive terrain a QA client needs: the grid dimensions
//! plus every openable door and weapon emplacement with its wire TOKEN. Without this
//! handout the open-door / enter- / exit-emplacement intents would be dead wire surface
//! (the GTW-694 ruling — a client can only target a token a view handed it).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::ids::{CellLevelNet, DoorToken, EmplacementToken};

/// The grid's **width** — its x span in cells. Mirror of the sim `GridWidth`
/// (whose inner is a `u8`, capped at `MAX_GRID_SPAN` = 60).
///
/// The wire inner is a deliberately widened `u16`: the protocol does not bind the
/// sim's current span cap into the wire format, so a future larger grid needs no
/// protocol-version bump. A private-inner newtype (no-bare-types), serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GridWidthNet(u16);

impl GridWidthNet {
    /// Build a grid width from its x span in cells.
    #[must_use]
    pub const fn new(width: u16) -> Self {
        Self(width)
    }
}

/// The grid's **height** — its y span in cells. Mirror of the sim `GridHeight`
/// (whose inner is a `u8`, capped at `MAX_GRID_SPAN` = 60).
///
/// The wire inner is a deliberately widened `u16` (see [`GridWidthNet`] for the
/// widening rationale). A private-inner newtype (no-bare-types), serde-transparent;
/// distinct from [`GridWidthNet`] (rule 3).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GridHeightNet(u16);

impl GridHeightNet {
    /// Build a grid height from its y span in cells.
    #[must_use]
    pub const fn new(height: u16) -> Self {
        Self(height)
    }
}

/// The grid's **storey count**. Mirror of the sim `GridLevels` (`u8`).
///
/// A private-inner newtype (no-bare-types), serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GridLevelsNet(u8);

impl GridLevelsNet {
    /// Build a grid storey count from its number of floors.
    #[must_use]
    pub const fn new(levels: u8) -> Self {
        Self(levels)
    }
}

/// The coarse grid **dimensions** — width × height × storey count, in cells.
///
/// The wire mirror of the sim `GridSize`. A named-field struct of typed spans. Serde
/// default shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridSizeNet {
    /// The grid's x span in cells.
    pub width:  GridWidthNet,
    /// The grid's y span in cells.
    pub height: GridHeightNet,
    /// The grid's storey count.
    pub levels: GridLevelsNet,
}

impl GridSizeNet {
    /// Build grid dimensions from a width, height, and storey count.
    #[must_use]
    pub const fn new(width: GridWidthNet, height: GridHeightNet, levels: GridLevelsNet) -> Self {
        Self {
            width,
            height,
            levels,
        }
    }
}

/// Whether a door is **open** — the wire mirror of the sim `DoorOpen`.
///
/// A private-inner newtype over `bool` (no-bare-types), serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DoorOpenNet(bool);

impl DoorOpenNet {
    /// Build a door-open answer — `true` open, `false` closed.
    #[must_use]
    pub const fn new(open: bool) -> Self {
        Self(open)
    }
}

/// Whether an emplacement is **manned** — the wire mirror of the sim
/// `EmplacementManned`.
///
/// A private-inner newtype over `bool` (no-bare-types), serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EmplacementMannedNet(bool);

impl EmplacementMannedNet {
    /// Build a manned answer — `true` occupied, `false` vacant.
    #[must_use]
    pub const fn new(manned: bool) -> Self {
        Self(manned)
    }
}

/// One openable **door** — its wire token, cell, and open state.
///
/// The [`token`](Self::token) is what [`NetIntent::OpenDoor`](crate::intent::NetIntent::OpenDoor)
/// echoes back. Serde default shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DoorView {
    /// The door's wire handle.
    pub token: DoorToken,
    /// The `(cell, level)` key the door sits at.
    pub at:    CellLevelNet,
    /// Whether the door is currently open.
    pub open:  DoorOpenNet,
}

impl DoorView {
    /// Build a door view from its token, cell, and open state.
    #[must_use]
    pub const fn new(token: DoorToken, at: CellLevelNet, open: DoorOpenNet) -> Self {
        Self { token, at, open }
    }
}

/// One weapon **emplacement** — its wire token, cell, and occupancy.
///
/// The [`token`](Self::token) is what the enter- / exit-emplacement
/// [`NetIntent`](crate::intent::NetIntent) variants echo back. Serde default shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmplacementView {
    /// The emplacement's wire handle.
    pub token:    EmplacementToken,
    /// The `(cell, level)` key the emplacement sits at.
    pub at:       CellLevelNet,
    /// Whether the emplacement is currently manned.
    pub occupied: EmplacementMannedNet,
}

impl EmplacementView {
    /// Build an emplacement view from its token, cell, and occupancy.
    #[must_use]
    pub const fn new(
        token: EmplacementToken,
        at: CellLevelNet,
        occupied: EmplacementMannedNet,
    ) -> Self {
        Self {
            token,
            at,
            occupied,
        }
    }
}

/// The terrain **summary** — the grid dimensions plus the interactive door /
/// emplacement token handout.
///
/// A curated DTO: not a full per-cell terrain dump (that is not QA-relevant), but the
/// grid bounds plus every door and emplacement with its wire token, so a client can
/// drive the door / emplacement intents. Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TerrainSummaryView {
    /// The coarse grid dimensions.
    pub grid:         GridSizeNet,
    /// Every openable door, with its wire token.
    pub doors:        Vec<DoorView>,
    /// Every weapon emplacement, with its wire token.
    pub emplacements: Vec<EmplacementView>,
}

impl TerrainSummaryView {
    /// Build a terrain summary from its grid dimensions and the door / emplacement
    /// handout.
    #[must_use]
    pub const fn new(
        grid: GridSizeNet,
        doors: Vec<DoorView>,
        emplacements: Vec<EmplacementView>,
    ) -> Self {
        Self {
            grid,
            doors,
            emplacements,
        }
    }
}
