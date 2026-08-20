//! Where a route starts, and which cells its start lets it leave by.

use bevy::prelude::Deref;

use crate::metric::CellLevel;

/// Whether one step out of a route's start cell is allowed.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepartureAdmitted(bool);

impl DepartureAdmitted {
    /// Wrap an admitted flag.
    #[must_use]
    pub const fn new(admitted: bool) -> Self {
        Self(admitted)
    }
}

/// A route's start cell, plus the only cells it may step to first when the start constrains them.
///
/// Only the first step is constrained; the rest of the route continues from wherever it landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Departure {
    from: CellLevel,
    only: Option<Vec<CellLevel>>,
}

impl Departure {
    /// Leave `from` by whichever neighbour the search prefers.
    #[must_use]
    pub const fn anywhere(from: CellLevel) -> Self {
        Self { from, only: None }
    }

    /// Leave `from` only by one of `cells`. An empty list strands the route on its start.
    #[must_use]
    pub const fn only_by(from: CellLevel, cells: Vec<CellLevel>) -> Self {
        Self {
            from,
            only: Some(cells),
        }
    }

    /// The cell the route starts on.
    #[must_use]
    pub const fn start(&self) -> CellLevel {
        self.from
    }

    /// Whether stepping from `origin` to `neighbour` is allowed.
    ///
    /// Every step that does not leave the start cell is allowed.
    #[must_use]
    pub fn admits(&self, origin: CellLevel, neighbour: CellLevel) -> DepartureAdmitted {
        let Some(only) = self.only.as_ref() else {
            return DepartureAdmitted::new(true);
        };
        if origin != self.from {
            return DepartureAdmitted::new(true);
        }
        DepartureAdmitted::new(only.contains(&neighbour))
    }
}
