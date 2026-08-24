//! What riding an emplacement adds to a move: where the route may leave, and what it costs.

use bevy::prelude::{Deref, Entity};

use crate::{
    acts::enter_emplacement::exit_emplacement_tu_cost,
    ganger::Tu,
    metric::CellLevel,
    pathfinder::Departure,
    terrain::emplacement::{
        EmplacementEntrySides, EmplacementFacing, Mounted, emplacement_entry_cells,
    },
    tuning::CombatTuning,
};

/// TU a walk adds because leaving a seat also pays the exit act.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DismountSurcharge(Tu);

impl DismountSurcharge {
    /// Nothing added: the mover rides no seat.
    pub const NONE: Self = Self(Tu::new(0));

    /// Wrap the TU that walking off a seat adds.
    #[must_use]
    pub const fn new(tu: Tu) -> Self {
        Self(tu)
    }
}

/// The exit act a walk also pays: the seat's cost while the mover rides one, nothing otherwise.
#[must_use]
pub fn dismount_surcharge(mounted: Option<&Mounted>, tuning: &CombatTuning) -> DismountSurcharge {
    seat_surcharge(mounted.and_then(Mounted::emplacement), tuning)
}

/// The same exit act, for a caller that already holds the seat entity rather than the mount.
#[must_use]
pub fn seat_surcharge(seat: Option<Entity>, tuning: &CombatTuning) -> DismountSurcharge {
    match seat {
        Some(_seat) => DismountSurcharge::new(exit_emplacement_tu_cost(tuning)),
        None => DismountSurcharge::NONE,
    }
}

/// A route off `from` that must leave by one of the seat's rotated entry cells.
/// The same set [`crate::acts::can_enter_emplacement`] tests for entry.
#[must_use]
pub fn seat_departure(
    from: CellLevel,
    seat: CellLevel,
    sides: Option<&EmplacementEntrySides>,
    facing: Option<&EmplacementFacing>,
) -> Departure {
    Departure::only_by(from, emplacement_entry_cells(seat, sides, facing).to_vec())
}
