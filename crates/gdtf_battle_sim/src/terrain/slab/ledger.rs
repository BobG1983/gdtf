//! Slab ledger resource: insert, peek, and deplete slab HP.

use bevy::{platform::collections::HashMap, prelude::Resource};

use crate::{
    metric::CellLevel,
    slab::types::{SlabDamage, SlabDestroyedFlag, SlabEntry, SlabEvent},
    tuning::SlabDefaults,
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

    /// Get the entry, seeding from a prototype if missing.
    pub fn entry_seeded(&mut self, key: CellLevel, prototype: SlabEntry) -> SlabEntry {
        *self.entries.entry(key).or_insert(SlabEntry::seeded(
            prototype.max_hp,
            prototype.armor_protection,
            prototype.armor_hardness,
        ))
    }

    /// Current HP, seeding if needed.
    pub fn current_for(&mut self, key: CellLevel, prototype: SlabEntry) -> super::SlabHp {
        self.entry_seeded(key, prototype).current_hp
    }

    /// Read-only view of an existing entry.
    #[must_use]
    pub fn peek(&self, key: &CellLevel) -> Option<&SlabEntry> {
        self.entries.get(key)
    }

    /// Default prototype from tuning defaults.
    #[must_use]
    pub fn prototype_for(_key: CellLevel, defaults: &SlabDefaults) -> SlabEntry {
        SlabEntry::seeded(
            defaults.hp(),
            defaults.armor_protection(),
            defaults.armor_hardness(),
        )
    }

    /// Apply damage; returns Damaged or Destroyed.
    pub fn deplete_slab(
        &mut self,
        cell_level: CellLevel,
        damage: SlabDamage,
        prototype: SlabEntry,
    ) -> SlabEvent {
        let mut entry = self.entry_seeded(cell_level, prototype);

        let remaining = entry.current_hp.saturating_sub(damage);
        entry.current_hp = remaining;
        if *remaining == 0 {
            entry.destroyed = SlabDestroyedFlag::new(true);
        }
        self.entries.insert(cell_level, entry);

        if *entry.destroyed {
            SlabEvent::Destroyed(cell_level)
        } else {
            SlabEvent::Damaged(cell_level)
        }
    }
}
