//! The ganger's grid-position component — the `(cell, level)` key it occupies.

use bevy::prelude::{Component, Deref};

use crate::metric::{Cell, CellLevel, Level};

/// A ganger's grid position — the `(cell, level)` key it occupies.
///
/// Wraps the E1.1 [`CellLevel`] (cell x/y + storey index) so a ganger's location
/// is the same `(cell, level)` identity the coarse occupancy and cover ledger are
/// keyed by (combat.md: "position everywhere is the pair `(cell, level)`"). A
/// distinct component from the rest of ganger state so movement systems can query
/// `&Position` / `&mut Position` alone.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position(CellLevel);

impl Position {
    /// Build a ganger position from the `(cell, level)` key it occupies.
    ///
    /// The one public constructor for the position component (private inner +
    /// constructor, the crate's newtype house style) — the move verbs and the
    /// occupancy-maintenance systems (E1.7) build a `Position` through this rather
    /// than reaching the private field.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self(at)
    }
}

/// `Default` (`Position` at the origin cell `(0, 0, 0)`) is a **spawn-seed
/// sentinel only** — the `bsn!`-scene spawn path's `get_or_insert_template`
/// seeds the component slot via `Default` before the authored `Position::new(..)`
/// overwrites it (GTW-322). It is NOT a meaningful authored position; a spawned
/// ganger always carries the situation's authored cell.
impl Default for Position {
    fn default() -> Self {
        Self::new(CellLevel::new(Cell::new(0, 0), Level::new(0)))
    }
}
