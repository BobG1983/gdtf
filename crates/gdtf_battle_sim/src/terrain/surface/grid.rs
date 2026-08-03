use bevy::{
    platform::collections::HashMap,
    prelude::{Deref, Resource},
};

use crate::{
    metric::{Cell, CellLevel},
    slab::SlabDestroyedFlag,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlabState {
        Present,
        Destroyed,
            Absent,
}

impl SlabState {
            #[must_use]
    pub const fn is_destroyed(self) -> SlabDestroyedFlag {
        SlabDestroyedFlag::new(matches!(self, Self::Destroyed))
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct GroundDamage(u32);

impl GroundDamage {
        #[must_use]
    pub const fn new(damage: u32) -> Self {
        Self(damage)
    }

                #[must_use]
    pub const fn accrue(self, amount: Self) -> Self {
        Self(self.0.saturating_add(amount.0))
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DamageApplied(bool);

impl DamageApplied {
        #[must_use]
    pub const fn new(applied: bool) -> Self {
        Self(applied)
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct SurfaceGrid {
                slabs:  HashMap<CellLevel, SlabState>,
            ground: HashMap<Cell, GroundDamage>,
}

impl SurfaceGrid {
            #[must_use]
    pub fn new() -> Self {
        Self {
            slabs:  HashMap::default(),
            ground: HashMap::default(),
        }
    }

                            #[must_use]
    pub fn slab_state(&self, key: &CellLevel) -> SlabState {
        self.slabs.get(key).copied().unwrap_or(SlabState::Absent)
    }

                                        pub fn set_slab(&mut self, key: CellLevel, state: SlabState) {
        if *self.slab_state(&key).is_destroyed() {
            return;
        }
        self.slabs.insert(key, state);
    }

                                            pub fn destroy_slab(&mut self, key: CellLevel) {
        self.slabs.insert(key, SlabState::Destroyed);
    }

            #[must_use]
    pub fn ground_damage(&self, cell: &Cell) -> GroundDamage {
        self.ground.get(cell).copied().unwrap_or_default()
    }

                                pub fn accrue_ground_damage(&mut self, cell: Cell, amount: GroundDamage) -> GroundDamage {
        let total = self.ground_damage(&cell).accrue(amount);
        self.ground.insert(cell, total);
        total
    }

                                            pub fn set_ground_damage(&mut self, cell: Cell, value: GroundDamage) -> DamageApplied {
        if value < self.ground_damage(&cell) {
            return DamageApplied::new(false);
        }
        self.ground.insert(cell, value);
        DamageApplied::new(true)
    }
}
