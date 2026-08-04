//! Adjacent-check helpers for downed actions.

use bevy::prelude::Deref;

use crate::{
    effects::bleed::BleedingOut,
    ganger::{Faction, LifeState, Position},
};

/// True when two positions share a level and are within one cell (8-way).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adjacent8(bool);

impl Adjacent8 {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(adjacent: bool) -> Self {
        Self(adjacent)
    }
}

/// Whether `a` and `b` are 8-adjacent on the same level.
#[must_use]
pub fn is_8_adjacent(a: Position, b: Position) -> Adjacent8 {
    let pa = **a;
    let pb = **b;
    if pa.z != pb.z {
        return Adjacent8::new(false);
    }
    let dx = (pa.x - pb.x).abs();
    let dy = (pa.y - pb.y).abs();
    Adjacent8::new(dx <= 1 && dy <= 1 && (dx != 0 || dy != 0))
}

/// Snapshot of the acting ganger for gate checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor {
    /// Position.
    pub pos:     Position,
    /// Life state.
    pub life:    LifeState,
    /// Faction.
    pub faction: Faction,
}

/// Snapshot of a downed target for gate checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownedTarget {
    /// Position.
    pub pos:          Position,
    /// Life state.
    pub life:         LifeState,
    /// Faction.
    pub faction:      Faction,
    /// Present when still bleeding out.
    pub bleeding_out: Option<BleedingOut>,
}
