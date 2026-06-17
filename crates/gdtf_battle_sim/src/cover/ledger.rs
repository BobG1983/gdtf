//! The [`CoverLedger`] resource — the sim's single authoritative store of cover
//! structural HP, one unified `(cell, level)` → [`CoverEntry`] map for BOTH walls
//! and props, lazily seeded and depleted marker-only.

use bevy::{platform::collections::HashMap, prelude::Resource};

use crate::{
    cover::{
        band::{BandFraction, band_for},
        types::{CoverDamage, CoverEntry, CoverEvent, CoverHp, Destroyed},
    },
    metric::CellLevel,
    tuning::CombatTuning,
};

/// The cover-HP ledger — the sim's **single authoritative writer** of cover-HP
/// state, one unified map for BOTH walls and scatter/props, keyed
/// [`CellLevel`] → [`CoverEntry`] (`docs/combat/resolution.md` §3; a
/// model-authoritative fact in the model/view split — ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`).
///
/// A Bevy [`Resource`] (NOT a component — there is one ledger per battle, not one
/// per cover entity), so the whole sim reads and spends cover HP through this one
/// store. Nothing else in the sim maintains a second cover-HP structure. The inner
/// map is **lazily seeded**: a `(cell, level)` that has taken no damage yet has no
/// map entry, and [`current_for`](CoverLedger::current_for) /
/// [`deplete_cover`](CoverLedger::deplete_cover) seed one with `current_hp ==
/// max_hp` on first access.
#[derive(Resource, Debug, Clone, Default)]
pub struct CoverLedger {
    /// The one unified `(cell, level)` → [`CoverEntry`] map for walls and props.
    /// Absent keys are lazily seeded on first access (C4) — an absent key is *not*
    /// "no cover", it is "cover at full HP that has taken no hit yet".
    entries: HashMap<CellLevel, CoverEntry>,
}

impl CoverLedger {
    /// Build an empty ledger (no cover has been touched yet — every piece is at
    /// full HP by lazy-seed).
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::default(),
        }
    }

    /// Insert (or replace) the authored cover entry at `key` — used at battle setup
    /// to register a wall or prop's `max_hp` / band / armor before any hit.
    ///
    /// This is how a piece's authored stats reach the ledger; it does **not**
    /// distinguish walls from props (the C2 one-ledger invariant — both go in the
    /// same map). Lazy seeding still applies to keys never inserted: a piece whose
    /// authored entry was registered seeds from *that* `max_hp`, while a key with
    /// no insert seeds from the caller-supplied prototype on access.
    pub fn insert(&mut self, key: CellLevel, entry: CoverEntry) {
        self.entries.insert(key, entry);
    }

    /// The cover entry at `key`, **lazily seeding** it from `prototype` (with
    /// `current_hp == max_hp`) if it is absent — the C4 first-access seed.
    ///
    /// `prototype` carries the piece's authored `max_hp` / band / armor; if `key`
    /// already has an entry (it has been inserted or already took a hit), that
    /// existing entry is returned unchanged and the prototype is ignored. Querying
    /// an absent entry therefore returns a freshly seeded entry with `current_hp ==
    /// max_hp`.
    pub fn entry_seeded(&mut self, key: CellLevel, prototype: CoverEntry) -> CoverEntry {
        *self.entries.entry(key).or_insert(CoverEntry::seeded(
            prototype.max_hp,
            prototype.height_band,
            prototype.armor_protection,
            prototype.armor_hardness,
        ))
    }

    /// The current HP at `key`, lazily seeding from `prototype` on first access —
    /// a read-flavored convenience over [`entry_seeded`](CoverLedger::entry_seeded).
    ///
    /// An absent `key` is seeded (so this returns `prototype.max_hp` for a piece
    /// that has taken no hit yet — the C4 invariant), then its `current_hp` is
    /// returned.
    pub fn current_for(&mut self, key: CellLevel, prototype: CoverEntry) -> CoverHp {
        self.entry_seeded(key, prototype).current_hp
    }

    /// A read-only peek at the entry already stored at `key`, **without** seeding —
    /// `None` if the piece has never been registered or hit.
    ///
    /// Use this when you must distinguish "already in the map" from "would be
    /// lazily seeded"; the seeding accessors are the normal read path.
    #[must_use]
    pub fn peek(&self, key: &CellLevel) -> Option<&CoverEntry> {
        self.entries.get(key)
    }

    /// Spend `damage` HP of the cover at `cell_level`, returning the
    /// [`CoverEvent`] outcome — the single system-facing depletion API
    /// (`docs/combat/resolution.md` §3's `apply_cover_hit`).
    ///
    /// Behavior (C5):
    /// 1. The entry is **lazily seeded** from `prototype` if absent (C4), so a
    ///    never-hit piece starts at `max_hp`.
    /// 2. `current_hp` is reduced by `damage` (saturating at zero — HP is a
    ///    non-negative pool).
    /// 3. If `current_hp` reaches zero, [`Destroyed`] is set `true` and a
    ///    [`CoverEvent::Destroyed`] carrying the [`CellLevel`] is returned;
    ///    otherwise [`CoverEvent::Damaged`] is returned.
    ///
    /// The destroyed-cover occupancy update + prop removal are **deferred to
    /// GTW-35** — this method emits the marker ONLY and does not act on it.
    ///
    /// `tuning` threads the combat-tuning resource through the system-facing
    /// signature (the spec'd `(cell_level, damage, tuning)` shape): the depletion
    /// arithmetic itself is data-free, but the band each `(cell, level)` occupies
    /// is classified from `tuning`'s [`BandEdge`](crate::tuning::BandEdge)
    /// level-fraction edges via [`band_for`](crate::cover::band_for), so any seeding
    /// that derives a band from a within-level fraction reads it from the SAME
    /// tuning — no band number is ever hardcoded here.
    pub fn deplete_cover(
        &mut self,
        cell_level: CellLevel,
        damage: CoverDamage,
        prototype: CoverEntry,
        tuning: &CombatTuning,
    ) -> CoverEvent {
        // Lazy-seed (C4) then re-classify the seeded entry's band from the SAME
        // tuning edges, so the stored band can never drift from `tuning` (C6) —
        // this also makes `tuning` a meaningfully-used parameter (no unused-param
        // warning) rather than a dropped one.
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
