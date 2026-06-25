//! The [`BraceStairCells`] battle-lifetime resource — the set of `(cell, level)`
//! positions that are the **LOWER endpoint** of an authored stair link, and therefore
//! have a stair-brace slab directly overhead (GTW-392).
//!
//! This is a **separate** set from the occupancy grid's `stair_cells` (which carries
//! BOTH endpoints of every stair link for LOS eye-lift + dual-cell presence,
//! GTW-391). [`BraceStairCells`] carries only the LOWER endpoint per stair link —
//! the only cell where a slab at `(x, y, level + 1)` is the genuine stair-brace slab
//! a kneeling occupant braces under. Computed at battle setup, inserted as a resource,
//! and removed at teardown alongside the other battle-lifetime resources.
//!
//! The fire path + HUD both consult this resource (via [`crate::terrain::slab`]'s
//! export) to decide brace eligibility — ensuring setup-time marker placement and
//! fire-time gating use the IDENTICAL lower-endpoint rule.

use bevy::{platform::collections::HashSet, prelude::Resource};

use crate::metric::CellLevel;

/// The authored **stair-brace cell** set — every `(cell, level)` that is the LOWER
/// endpoint of a [`Stair`](crate::vertical::LinkKind::Stair) link in the situation's
/// vertical-link graph (GTW-392).
///
/// A Bevy [`Resource`] (one set per battle, inserted at setup by
/// [`setup_battle`](crate::situation::setup_battle), removed at teardown). A named
/// newtype over `HashSet<CellLevel>` (no-bare-types: the brace-stair cell set is a
/// domain value, not a bare `HashSet`). It derefs read-only to the underlying set
/// via `contains`; there is no mutable `Deref` — the set is frozen after setup.
///
/// This is DISTINCT from the [`OccupancyGrid`](crate::occupancy::OccupancyGrid)'s
/// `stair_cells` field (which holds BOTH endpoints for LOS/presence). The lower-endpoint
/// restriction is what guarantees only the structurally correct slab (the one at
/// `lower_stair.level + 1`) earns the brace, not the ceiling at the upper arrival cell.
#[derive(Resource, Debug, Clone, Default)]
pub struct BraceStairCells(HashSet<CellLevel>);

impl BraceStairCells {
    /// Build a [`BraceStairCells`] resource from the precomputed lower-endpoint set.
    ///
    /// Called at battle setup with the set already narrowed to lower endpoints — the
    /// caller ([`setup_battle`](crate::situation::setup_battle)) computes the lower
    /// endpoints via `min(from.z, to.z)` per stair link and passes the result here.
    #[must_use]
    pub const fn new(cells: HashSet<CellLevel>) -> Self {
        Self(cells)
    }

    /// Build an empty [`BraceStairCells`] resource — for a situation with no stair
    /// links, or for contexts (e.g. HUD fail-closed) where the resource is absent.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Whether `cell_level` is a brace-eligible stair cell — the LOWER endpoint of
    /// some authored stair link, beneath whose overhead slab a kneeling occupant braces.
    ///
    /// Returns `true` when the cell was registered at battle setup as a lower stair
    /// endpoint. A cell absent from the set — including every cell in an empty
    /// [`BraceStairCells`] — returns `false` (no brace).
    #[must_use]
    pub fn contains(&self, cell_level: &CellLevel) -> bool {
        self.0.contains(cell_level)
    }
}
