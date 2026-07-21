//! [`FogView`] — the player squad's fog-of-war summary (GTW-734, GTW-763).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// The count of `(cell, level)` keys currently **visible** to the player squad. Mirror
/// of `SquadVisibility::visible_cells().count()` (`u32`).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VisibleCellCountNet(u32);

impl VisibleCellCountNet {
    /// Build a visible-cell count from its total.
    #[must_use]
    pub const fn new(count: u32) -> Self {
        Self(count)
    }
}

/// The count of `(cell, level)` keys the player squad has ever **explored**. Mirror of
/// `SquadVisibility::explored_cells().count()` (`u32`); a distinct newtype from
/// [`VisibleCellCountNet`] (no-bare-types rule 3 — the current sightline is not the
/// cumulative explored set).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExploredCellCountNet(u32);

impl ExploredCellCountNet {
    /// Build an explored-cell count from its total.
    #[must_use]
    pub const fn new(count: u32) -> Self {
        Self(count)
    }
}

/// The player squad's **fog** snapshot — how many `(cell, level)` keys are currently
/// visible and how many have ever been explored.
///
/// A curated summary of the sim `SquadVisibility` carried as two COUNTS, not the two full
/// cell lists. The counts answer the real QA signal — fog is limiting sight exactly when
/// [`visible_count`](Self::visible_count) is below the grid's cell total, and the explored
/// set grows as the squad advances — while keeping the snapshot a fixed size. The prior
/// per-cell lists grew with the squad's sightline (thousands of cells on an open map, the
/// explored set growing every turn), which overflowed a QA client's context. `visible` is
/// always `<=` `explored` (visible implies explored). See GTW-763.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FogView {
    /// How many `(cell, level)` keys are currently visible to the player squad.
    pub visible_count:  VisibleCellCountNet,
    /// How many `(cell, level)` keys the player squad has ever seen.
    pub explored_count: ExploredCellCountNet,
}

impl FogView {
    /// Build a fog view from its visible + explored cell counts.
    #[must_use]
    pub const fn new(
        visible_count: VisibleCellCountNet,
        explored_count: ExploredCellCountNet,
    ) -> Self {
        Self {
            visible_count,
            explored_count,
        }
    }
}
