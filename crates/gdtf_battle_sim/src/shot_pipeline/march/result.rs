//! What a march found and where.

use bevy::prelude::Entity;

use crate::{
    cover::{CoverEntry, HeightBand},
    metric::{CellLevel, SimPos},
};

/// Kind of impact a march produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarchKind {
    /// Living combatant.
    Ganger(Entity),
    /// Cover entry.
    Cover(CoverEntry),
    /// Floor slab.
    Slab,
    /// Open ground.
    Ground,
    /// Nothing solid; ray left the map or expired.
    Miss,
}

/// Full result of one ray march.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarchResult {
    /// What was hit.
    pub kind:   MarchKind,
    /// Cell and level of impact.
    pub at:     CellLevel,
    /// Height band at impact.
    pub band:   HeightBand,
    /// World position of impact.
    pub impact: SimPos,
}
