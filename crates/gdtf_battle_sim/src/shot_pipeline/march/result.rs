//! The march verdict types — the public outcome a [`march_vector`](super::march_vector)
//! reports: what the round failed to clear ([`MarchKind`]) and the full
//! [`MarchResult`] payload (`docs/combat/resolution.md` §2).

use bevy::prelude::Entity;

use crate::{
    cover::{CoverEntry, HeightBand},
    metric::{CellLevel, SimPos},
};

/// What the round **failed to clear** — the kind of thing the march stopped on, with
/// the payload that identifies it (`docs/combat/resolution.md` §2: "the first thing
/// the round fails to clear: (ganger | cover | floor/roof slab | ground) — or a
/// clean miss off the grid").
///
/// A named domain enum (no-bare-types: the march verdict is a domain value, not a
/// bare tag) carrying the struck object itself where there is one — a ganger's
/// [`Entity`] handle (NEVER a numeric id — GTW-10 / GTW-12) and the cover's
/// [`CoverEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarchKind {
    /// The round impacted a **ganger** occupant — carries its [`Entity`] handle (the
    /// struck actor; never a numeric id).
    Ganger(Entity),
    /// The round impacted a piece of **cover** — carries the [`CoverEntry`] read from
    /// the [`CoverLedger`](crate::cover::CoverLedger) for the struck `(cell, level)`.
    Cover(CoverEntry),
    /// The round was stopped by an intact floor/roof **slab** at a z-boundary.
    Slab,
    /// The round left the **bottom** of the grid and struck the ground (damaged,
    /// never destroyed — crater FX later).
    Ground,
    /// A clean **miss** — the round left the grid off the top or laterally without
    /// failing to clear anything.
    Miss,
}

/// The outcome of a [`march_vector`](super::march_vector) — the first thing the round
/// fails to clear, the `(cell, level)` it happened at, the crossed-cell [`HeightBand`]
/// of the round, and the impact point (`docs/combat/resolution.md` §2).
///
/// Every field is a named domain value (no-bare-types): the [`MarchKind`] verdict
/// (with its struck-object payload), the [`CellLevel`] grid key, the round's
/// [`HeightBand`] at the crossing, and the [`SimPos`] impact point — a continuous
/// sim-unit position built via [`SimPos::new`] (AC #7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarchResult {
    /// What the round failed to clear (with the struck ganger / cover payload).
    pub kind:   MarchKind,
    /// The `(cell, level)` the round failed to clear at (the exit cell for a
    /// ground / lateral / top result).
    pub at:     CellLevel,
    /// The round's clearance band at the crossed cell
    /// ([`round_band_for_cell`](crate::clearance::round_band_for_cell)).
    pub band:   HeightBand,
    /// The impact point in sim units — where along the ray the round stopped (or
    /// exited), built via [`SimPos::new`].
    pub impact: SimPos,
}
