//! `SurfaceGrid` resource and related surface state types.

use bevy::{
    platform::collections::HashMap,
    prelude::{Deref, Resource},
};

use crate::metric::{Cell, CellLevel};

/// Whether a slab stands at a cell, or no slab does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlabState {
    /// Intact slab.
    Present,
    /// No slab stands here.
    Absent,
}

/// Accrued damage to open ground at a cell.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct GroundDamage(u32);

impl GroundDamage {
    /// Wrap a damage amount.
    #[must_use]
    pub const fn new(damage: u32) -> Self {
        Self(damage)
    }

    /// Add more damage without overflow.
    #[must_use]
    pub const fn accrue(self, amount: Self) -> Self {
        Self(self.0.saturating_add(amount.0))
    }
}

/// Whether a ground-damage write was accepted.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DamageApplied(bool);

impl DamageApplied {
    /// Wrap an applied flag.
    #[must_use]
    pub const fn new(applied: bool) -> Self {
        Self(applied)
    }
}

/// Authoritative surface state: slab presence and ground damage.
#[derive(Resource, Debug, Clone, Default)]
pub struct SurfaceGrid {
    slabs:  HashMap<CellLevel, SlabState>,
    ground: HashMap<Cell, GroundDamage>,
}

impl SurfaceGrid {
    /// Empty grid.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slabs:  HashMap::default(),
            ground: HashMap::default(),
        }
    }

    /// Slab state at a cell (Absent if never set).
    #[must_use]
    pub fn slab_state(&self, key: &CellLevel) -> SlabState {
        self.slabs.get(key).copied().unwrap_or(SlabState::Absent)
    }

    /// Set slab state, in either direction.
    pub fn set_slab(&mut self, key: CellLevel, state: SlabState) {
        self.slabs.insert(key, state);
    }

    /// Current ground damage at a cell.
    #[must_use]
    pub fn ground_damage(&self, cell: &Cell) -> GroundDamage {
        self.ground.get(cell).copied().unwrap_or_default()
    }

    /// Add ground damage and return the new total.
    pub fn accrue_ground_damage(&mut self, cell: Cell, amount: GroundDamage) -> GroundDamage {
        let total = self.ground_damage(&cell).accrue(amount);
        self.ground.insert(cell, total);
        total
    }

    /// Set ground damage only if the new value is not lower than the current.
    pub fn set_ground_damage(&mut self, cell: Cell, value: GroundDamage) -> DamageApplied {
        if value < self.ground_damage(&cell) {
            return DamageApplied::new(false);
        }
        self.ground.insert(cell, value);
        DamageApplied::new(true)
    }
}
