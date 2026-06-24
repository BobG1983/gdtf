//! The [`SlabLedger`] resource — the sim's single authoritative store of slab
//! structural HP, one unified `(cell, level)` → [`SlabEntry`] map, lazily seeded and
//! depleted marker-only. Mirrors the GTW-364 [`CoverLedger`](crate::cover::CoverLedger).

use bevy::{platform::collections::HashMap, prelude::Resource};

use crate::{
    metric::CellLevel,
    slab::types::{SlabDamage, SlabDestroyedFlag, SlabEntry, SlabEvent},
    tuning::SlabDefaults,
};

/// The slab-HP ledger — the sim's **single authoritative writer** of slab-HP state,
/// one unified map keyed [`CellLevel`] → [`SlabEntry`] for every floor/roof slab
/// (`docs/combat/resolution.md` §3.1; a model-authoritative fact in the model/view
/// split — ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`; user-ruled
/// 2026-06-22). The slab mirror of [`CoverLedger`](crate::cover::CoverLedger).
///
/// A Bevy [`Resource`] (NOT a component — there is one ledger per battle, not one per
/// slab), so the whole sim reads and spends slab HP through this one store. The inner
/// map is **lazily seeded**: a `(cell, level)` that has taken no damage yet has no map
/// entry, and [`current_for`](SlabLedger::current_for) /
/// [`deplete_slab`](SlabLedger::deplete_slab) seed one with `current_hp == max_hp`
/// from the [`SlabDefaults`] **tuning** prototype on first access (C4 / C7) — slabs
/// are uniform level structure (authored as a bare `(cell, level)` list with NO
/// per-slab HP), so the seed magnitude comes from the genuinely-consumed combat-tuning
/// leaf, NOT a per-piece authored value the way cover's does.
#[derive(Resource, Debug, Clone, Default)]
pub struct SlabLedger {
    /// The one unified `(cell, level)` → [`SlabEntry`] map. Absent keys are lazily
    /// seeded on first access (C4) — an absent key is *not* "no slab", it is "a slab
    /// at full HP that has taken no hit yet" (existence is the [`SurfaceGrid`](crate::surface::SurfaceGrid)'s
    /// concern; this ledger holds only the HP/armor of a slab that gets struck).
    entries: HashMap<CellLevel, SlabEntry>,
}

impl SlabLedger {
    /// Build an empty ledger (no slab has been touched yet — every slab is at full HP
    /// by lazy-seed from [`SlabDefaults`]).
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::default(),
        }
    }

    /// Insert (or replace) the slab entry at `key` — the test/programmatic seam to
    /// register a slab's `max_hp` / armor before any hit (mirrors
    /// [`CoverLedger::insert`](crate::cover::CoverLedger::insert)).
    ///
    /// Battle setup does NOT call this for shipped slabs (they lazy-seed from
    /// [`SlabDefaults`] tuning, since slabs carry no authored per-piece HP); it exists
    /// so a test can pin a low-HP slab for a breach scenario without routing through
    /// the tuning prototype. A key with no insert seeds from the caller-supplied
    /// prototype on first access.
    pub fn insert(&mut self, key: CellLevel, entry: SlabEntry) {
        self.entries.insert(key, entry);
    }

    /// The slab entry at `key`, **lazily seeding** it from `prototype` (with
    /// `current_hp == max_hp`) if it is absent — the C4 first-access seed.
    ///
    /// `prototype` carries the slab's `max_hp` / armor (from [`SlabDefaults`] tuning at
    /// the call site); if `key` already has an entry (it has been inserted or already
    /// took a hit), that existing entry is returned unchanged and the prototype is
    /// ignored. Querying an absent entry therefore returns a freshly seeded entry with
    /// `current_hp == max_hp`.
    pub fn entry_seeded(&mut self, key: CellLevel, prototype: SlabEntry) -> SlabEntry {
        *self.entries.entry(key).or_insert(SlabEntry::seeded(
            prototype.max_hp,
            prototype.armor_protection,
            prototype.armor_hardness,
        ))
    }

    /// The current HP at `key`, lazily seeding from `prototype` on first access — a
    /// read-flavored convenience over [`entry_seeded`](SlabLedger::entry_seeded).
    ///
    /// An absent `key` is seeded (so this returns `prototype.max_hp` for a slab that
    /// has taken no hit yet — the C4 invariant), then its `current_hp` is returned.
    pub fn current_for(&mut self, key: CellLevel, prototype: SlabEntry) -> super::SlabHp {
        self.entry_seeded(key, prototype).current_hp
    }

    /// A read-only peek at the entry already stored at `key`, **without** seeding —
    /// `None` if the slab has never been registered or hit.
    ///
    /// Use this when you must distinguish "already in the map" from "would be lazily
    /// seeded"; the seeding accessors are the normal read path.
    #[must_use]
    pub fn peek(&self, key: &CellLevel) -> Option<&SlabEntry> {
        self.entries.get(key)
    }

    /// The seed prototype for a lazily-seeded slab at `key` — built from the
    /// [`SlabDefaults`] **tuning** leaf (C7), since slabs are uniform level structure
    /// with no per-piece authored HP.
    ///
    /// This is the genuinely-consumed read of the tuning seam: every depletion of a
    /// never-before-hit slab seeds its `max_hp` / armor from `defaults` here, so the
    /// `.ron` leaf is on the live path (NOT a dead leaf). The `key` parameter is
    /// reserved for a future per-level / per-position prototype variation; today the
    /// defaults are uniform, so it is unused — kept in the signature so the seam reads
    /// the same shape as the cover prototype (`(key, defaults) → entry`).
    #[must_use]
    pub fn prototype_for(_key: CellLevel, defaults: &SlabDefaults) -> SlabEntry {
        SlabEntry::seeded(
            defaults.hp(),
            defaults.armor_protection(),
            defaults.armor_hardness(),
        )
    }

    /// Spend `damage` HP of the slab at `cell_level`, returning the [`SlabEvent`]
    /// outcome — the single system-facing depletion API (the slab mirror of
    /// [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover)).
    ///
    /// Behavior (C4):
    /// 1. The entry is **lazily seeded** from `prototype` if absent, so a never-hit
    ///    slab starts at `max_hp` (the [`SlabDefaults`] tuning value).
    /// 2. `current_hp` is reduced by `damage` (saturating at zero — HP is a
    ///    non-negative pool).
    /// 3. If `current_hp` reaches zero, [`SlabDestroyedFlag`] is set `true` and a
    ///    [`SlabEvent::Destroyed`] carrying the [`CellLevel`] is returned; otherwise
    ///    [`SlabEvent::Damaged`] is returned.
    ///
    /// The HP pool is **persistent across strikes** (C4): a second hit on a slab this
    /// method already damaged reads the reduced `current_hp` from the map (it is
    /// re-inserted each call), so a damaged slab can be finished by a later round. The
    /// surface-grid `destroy_slab` + LOS rebuild are the fire path's bridge — this
    /// method emits the marker ONLY.
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
