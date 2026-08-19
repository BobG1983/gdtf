//! Slab ledger resource: insert, peek, and deplete slab HP.

use bevy::{platform::collections::HashMap, prelude::Resource};

use crate::{
    metric::CellLevel,
    slab::types::{SlabDamage, SlabDestroyedFlag, SlabEntry, SlabEvent},
};

/// Authoritative store of slab structural HP by cell.
#[derive(Resource, Debug, Clone, Default)]
pub struct SlabLedger {
    entries: HashMap<CellLevel, SlabEntry>,
}

impl SlabLedger {
    /// Empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::default(),
        }
    }

    /// Insert or replace a slab entry.
    pub fn insert(&mut self, key: CellLevel, entry: SlabEntry) {
        self.entries.insert(key, entry);
    }

    /// Read-only view of an existing entry.
    #[must_use]
    pub fn peek(&self, key: &CellLevel) -> Option<&SlabEntry> {
        self.entries.get(key)
    }

    /// Apply damage to an authored slab; `None` at a cell with no entry, which mints nothing.
    pub fn deplete_slab(&mut self, cell_level: CellLevel, damage: SlabDamage) -> Option<SlabEvent> {
        let mut entry = *self.entries.get(&cell_level)?;

        let remaining = entry.current_hp.saturating_sub(damage);
        entry.current_hp = remaining;
        if *remaining == 0 {
            entry.destroyed = SlabDestroyedFlag::new(true);
        }
        self.entries.insert(cell_level, entry);

        if *entry.destroyed {
            Some(SlabEvent::Destroyed(cell_level))
        } else {
            Some(SlabEvent::Damaged(cell_level))
        }
    }
}
