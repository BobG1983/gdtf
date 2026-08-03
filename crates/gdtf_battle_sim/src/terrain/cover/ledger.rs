//! Cover ledger resource: insert, peek, and deplete cover HP.

use bevy::{platform::collections::HashMap, prelude::Resource};

use crate::{
    cover::{
        band::{BandFraction, band_for},
        types::{CoverDamage, CoverEntry, CoverEvent, CoverHp, Destroyed},
    },
    metric::CellLevel,
    tuning::CombatTuning,
};

/// Authoritative store of cover structural HP by cell.
#[derive(Resource, Debug, Clone, Default)]
pub struct CoverLedger {
    entries: HashMap<CellLevel, CoverEntry>,
}

impl CoverLedger {
    /// Empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::default(),
        }
    }

    /// Insert or replace a cover entry.
    pub fn insert(&mut self, key: CellLevel, entry: CoverEntry) {
        self.entries.insert(key, entry);
    }

    /// Get the entry, seeding from a prototype if missing.
    pub fn entry_seeded(&mut self, key: CellLevel, prototype: CoverEntry) -> CoverEntry {
        *self.entries.entry(key).or_insert(CoverEntry::seeded(
            prototype.max_hp,
            prototype.height_band,
            prototype.armor_protection,
            prototype.armor_hardness,
        ))
    }

    /// Current HP, seeding if needed.
    pub fn current_for(&mut self, key: CellLevel, prototype: CoverEntry) -> CoverHp {
        self.entry_seeded(key, prototype).current_hp
    }

    /// Read-only view of an existing entry.
    #[must_use]
    pub fn peek(&self, key: &CellLevel) -> Option<&CoverEntry> {
        self.entries.get(key)
    }

    /// Apply damage; returns Damaged or Destroyed.
    pub fn deplete_cover(
        &mut self,
        cell_level: CellLevel,
        damage: CoverDamage,
        prototype: CoverEntry,
        tuning: &CombatTuning,
    ) -> CoverEvent {
        let mut entry = self.entry_seeded(cell_level, prototype);
        entry.height_band = band_for(BandFraction::from_band(entry.height_band, tuning), tuning);

        let remaining = entry.current_hp.saturating_sub(damage);
        entry.current_hp = remaining;
        if *remaining == 0 {
            entry.destroyed = Destroyed::new(true);
        }
        self.entries.insert(cell_level, entry);

        if *entry.destroyed {
            CoverEvent::Destroyed(cell_level)
        } else {
            CoverEvent::Damaged(cell_level)
        }
    }
}
