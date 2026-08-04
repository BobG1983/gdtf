//! Trait for fanning on-death effects at a cell.

use bevy::prelude::Query;

use crate::{
    effects::{
        fields::{FieldDefRegistry, FieldRegistry},
        on_death::OnDeathOccurred,
    },
    ganger::{Hp, LifeState},
    metric::CellLevel,
    occupancy::OccupancyGrid,
};

/// Mutable HP and life state for a victim.
pub type VictimRow = (&'static mut Hp, &'static mut LifeState);

/// Context passed when an on-death effect fans out.
pub struct DeathFanOut<'a, 'w, 's> {
    /// Occupancy grid.
    pub grid:       &'a OccupancyGrid,
    /// Victim query.
    pub victims:    &'a mut Query<'w, 's, VictimRow>,
    /// Live field placements.
    pub fields:     &'a mut FieldRegistry,
    /// Field catalog, if loaded.
    pub field_defs: Option<&'a FieldDefRegistry>,
    /// Cascade queue for chained deaths.
    pub cascade:    &'a mut Vec<OnDeathOccurred>,
}

/// Behaviour for an on-death effect.
pub trait ApplyOnDeathEffect {
    /// Apply this effect at a cell.
    fn fan_at(&self, at: CellLevel, fan_out: &mut DeathFanOut<'_, '_, '_>);
}
