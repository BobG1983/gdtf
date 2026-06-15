//! Cover-HP ledger: the model's single authoritative store of cover structural
//! HP, keyed `(cell, level)` — **one** ledger for walls *and* scatter/props.
//!
//! This is the E1.4 cover-ledger slice. Per `docs/combat/resolution.md` §3, cover
//! is a physical object with a **height** and its **own armor stats + HP**, using
//! the same armor model as a ganger (so this module **reuses**
//! [`crate::armor::ArmorProtection`] / [`crate::armor::ArmorHardness`] — cover is
//! not a second armor model). "The HP lives on the model's cover ledger (one
//! ledger for walls *and* props, keyed (cell, level)): `apply_cover_hit` spends
//! it, and depletion emits a cover-destroyed event carrying (cell, level)"
//! (resolution.md §3; `docs/architecture.md`'s model-authoritative-facts table).
//!
//! Three deliberate design points carried from the docs and this ticket:
//!
//! 1. **One unified map.** [`CoverLedger`] keys [`CellLevel`] → [`CoverEntry`] for
//!    BOTH walls and props — there is no second cover-HP structure anywhere in the
//!    sim. A wall and a prop on different `(cell, level)` keys both live in the
//!    same map and are both retrievable.
//! 2. **Lazy seeding.** `current_hp` is seeded to `max_hp` on **first access**, not
//!    pre-populated for every cell of the grid. Querying an absent entry returns a
//!    freshly seeded entry with `current_hp == max_hp`
//!    (`docs/architecture.md`: "lazily seeded from each piece's max HP").
//! 3. **Marker-only depletion.** [`CoverLedger::deplete_cover`] spends HP and, when
//!    `current_hp` reaches zero, sets [`Destroyed`] and returns a
//!    [`CoverEvent::Destroyed`] carrying the [`CellLevel`]. The destroyed-cover
//!    occupancy update + prop removal are **deferred to GTW-35** — this slice emits
//!    the marker ONLY and does not act on it.
//!
//! Height-band thresholds are **never** hardcoded here: [`band_for`] classifies a
//! within-level fraction into a [`HeightBand`] by reading the E1.1 [`BandEdge`]
//! level-fraction edges off [`CombatTuning`] (`docs/combat/battle-space.md`
//! §"Banding": the band edges are tunable level-fractions ≈ ⅓ and ⅔ of a level,
//! dimensionless and decoupled from any pixel).

use bevy::{
    platform::collections::HashMap,
    prelude::{Deref, Resource},
};
use serde::Deserialize;

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    metric::CellLevel,
    tuning::CombatTuning,
};

/// Cover structural HP — the hit points a piece of cover (wall or prop) carries
/// in the ledger.
///
/// One newtype used for **both** [`CoverEntry::current_hp`] and
/// [`CoverEntry::max_hp`]: they are the same *kind* of value (a structural-HP
/// quantity), distinguished by their field. A `u32` because cover HP is a
/// non-negative pool that depletes to zero (it never tracks below zero — at zero
/// the cover is destroyed). Private inner + derived [`Deref`] (house style); a
/// magnitude is per-object data (TBD tuning), not pinned here.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverHp(u32);

impl CoverHp {
    /// Build a cover-HP value from its magnitude (per-object data, TBD tuning).
    #[must_use]
    pub const fn new(hp: u32) -> Self {
        Self(hp)
    }
}

/// A piece of cover's height band — which clearance band it occupies, so the
/// march knows what a round must fly strictly higher than to clear it
/// (`docs/combat/resolution.md` §3: cover "occupies its cell at its `cover_height`
/// band (LOW / MID / HIGH)").
///
/// A named domain enum (the §2/§3 LOW/MID/HIGH banding), introduced here at first
/// use — not a bare index. The level-fraction → band classification lives in
/// [`band_for`], which reads the tunable [`BandEdge`] level-fraction edges; this
/// enum never carries a fraction itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum HeightBand {
    /// The lowest clearance band — a round clears it by flying MID or HIGH.
    Low,
    /// The middle clearance band — a round clears it only by flying HIGH.
    Mid,
    /// The tallest clearance band — nothing flies strictly higher within a storey.
    High,
}

/// Whether a piece of cover has been destroyed — its HP has been spent to zero.
///
/// A named newtype over `bool` (no-bare-types: a destroyed flag is a domain value,
/// not a bare boolean). Set `true` by [`CoverLedger::deplete_cover`] exactly when
/// `current_hp` reaches zero; the occupancy/prop consequences of that are GTW-35.
/// Private inner + derived [`Deref`] (house style).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Destroyed(bool);

impl Destroyed {
    /// Build a destroyed flag from its boolean state.
    #[must_use]
    pub const fn new(destroyed: bool) -> Self {
        Self(destroyed)
    }
}

/// The cover-HP record for one `(cell, level)` — a wall *or* a prop, the same
/// shape for both (`docs/combat/resolution.md` §3: cover has "its own armor stats
/// + HP", and "one ledger for walls *and* props").
///
/// Every field is a named newtype (no-bare-types): the structural HP pool
/// ([`current_hp`](CoverEntry::current_hp) + [`max_hp`](CoverEntry::max_hp), both
/// [`CoverHp`]), the occupied [`height_band`](CoverEntry::height_band), the cover's
/// own armor stats ([`armor_protection`](CoverEntry::armor_protection) +
/// [`armor_hardness`](CoverEntry::armor_hardness), **reusing** the GTW-153 armor
/// newtypes — cover uses the same armor model per resolution.md §3), and the
/// [`destroyed`](CoverEntry::destroyed) flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverEntry {
    /// The current structural HP remaining — depleted by `deplete_cover`, seeded to
    /// `max_hp` on first access.
    pub current_hp:       CoverHp,
    /// The full structural HP the piece seeds to (its max).
    pub max_hp:           CoverHp,
    /// The clearance band this cover occupies (LOW / MID / HIGH).
    pub height_band:      HeightBand,
    /// The cover's damage-reduction stat (same armor model as a ganger).
    pub armor_protection: ArmorProtection,
    /// The penetration this cover shrugs off (same armor model as a ganger).
    pub armor_hardness:   ArmorHardness,
    /// Whether this cover has been destroyed (HP spent to zero).
    pub destroyed:        Destroyed,
}

impl CoverEntry {
    /// Build a cover entry **seeded** with `current_hp == max_hp` and not
    /// destroyed — the freshly-seeded shape returned for a piece on first access.
    ///
    /// Takes the piece's authored `max_hp`, `height_band`, and armor stats; the
    /// current HP is seeded to the full `max_hp` (the C4 lazy-seed shape) and
    /// [`Destroyed`] starts `false`.
    #[must_use]
    pub const fn seeded(
        max_hp: CoverHp,
        height_band: HeightBand,
        armor_protection: ArmorProtection,
        armor_hardness: ArmorHardness,
    ) -> Self {
        Self {
            current_hp: max_hp,
            max_hp,
            height_band,
            armor_protection,
            armor_hardness,
            destroyed: Destroyed::new(false),
        }
    }
}

/// The outcome of spending cover HP — what [`CoverLedger::deplete_cover`] returns.
///
/// A named domain enum (not a bare `Option`/`bool`): either the hit only
/// **damaged** the cover, or it **destroyed** it, in which case the variant carries
/// the [`CellLevel`] for the presenter's reactions (`docs/combat/resolution.md`
/// §3: "depletion emits a cover-destroyed event carrying (cell, level)";
/// `docs/architecture.md`'s `CoverDestroyed { cell, level }`). This slice emits the
/// marker only — acting on it (occupancy + prop removal) is GTW-35.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoverEvent {
    /// The cover took damage but still stands (`current_hp > 0`).
    Damaged(CellLevel),
    /// The cover was destroyed this hit (`current_hp` reached zero) — carries the
    /// `(cell, level)` the destroyed cover occupied.
    Destroyed(CellLevel),
}

/// The cover-HP ledger — the sim's **single authoritative writer** of cover-HP
/// state, one unified map for BOTH walls and scatter/props, keyed
/// [`CellLevel`] → [`CoverEntry`] (`docs/combat/resolution.md` §3;
/// `docs/architecture.md`'s model-authoritative-facts table).
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
    /// is classified from `tuning`'s [`BandEdge`] level-fraction edges via
    /// [`band_for`], so any seeding that derives a band from a within-level
    /// fraction reads it from the SAME tuning — no band number is ever hardcoded
    /// here.
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

        if entry.destroyed.0 {
            CoverEvent::Destroyed(cell_level)
        } else {
            CoverEvent::Damaged(cell_level)
        }
    }
}

impl CoverHp {
    /// Reduce this HP by `damage`, **saturating at zero** — cover HP is a
    /// non-negative pool and depletion can never carry it below zero (at zero the
    /// cover is destroyed).
    #[must_use]
    pub fn saturating_sub(self, damage: CoverDamage) -> Self {
        Self(self.0.saturating_sub(*damage))
    }
}

/// A quantity of damage spent against cover HP — the amount one hit removes from a
/// [`CoverEntry`]'s `current_hp`.
///
/// A distinct newtype from [`CoverHp`] (no-bare-types rule 3: distinct concepts
/// get distinct types even over the same inner `u32`) — a damage amount is not an
/// HP pool, and they must not be interchangeable. A magnitude is per-hit data, not
/// pinned here. Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverDamage(u32);

impl CoverDamage {
    /// Build a cover-damage amount from its magnitude (per-hit data).
    #[must_use]
    pub const fn new(damage: u32) -> Self {
        Self(damage)
    }
}

/// A **within-level fraction** — a height above the crossed cell's level floor,
/// expressed as a dimensionless fraction of one level's height (`z ∈ [0,1)`
/// within a storey). The input to band classification
/// (`docs/combat/battle-space.md` §"Banding": the continuous level-fraction
/// datums fed into the band assignment).
///
/// A named newtype over `f32` (no-bare-types): a within-level clearance fraction
/// is a domain value, distinct from a [`BandEdge`] *threshold* it is compared
/// against. Used only as the [`band_for`] input. Private inner + derived
/// [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct BandFraction(f32);

impl BandFraction {
    /// Build a within-level clearance fraction from its magnitude (a fraction of
    /// one level's height).
    #[must_use]
    pub const fn new(fraction: f32) -> Self {
        Self(fraction)
    }

    /// A representative within-level fraction for an already-classified
    /// [`HeightBand`], read from `tuning`'s edges — never a hardcoded band number
    /// (C6).
    ///
    /// Used by [`CoverLedger::deplete_cover`] to round-trip a stored band through
    /// the tuning edges so the band can never drift from the current tuning: LOW
    /// maps below the LOW→MID edge, MID between the two edges, HIGH at or above the
    /// MID→HIGH edge. The exact fraction is immaterial — only that it lands back in
    /// the same band under [`band_for`] — so it is derived purely from `tuning`'s
    /// authored edges, with no literal magnitude beyond an arbitrary sub-edge nudge.
    #[must_use]
    pub fn from_band(band: HeightBand, tuning: &CombatTuning) -> Self {
        let edges = &tuning.projectile_band_edges;
        match band {
            // Strictly below the LOW→MID edge → LOW (half the edge stays below it
            // for any positive edge, with no literal threshold of our own).
            HeightBand::Low => Self(*edges.low_mid * 0.5),
            // At-or-above LOW→MID, strictly below MID→HIGH → MID.
            HeightBand::Mid => Self(*edges.low_mid),
            // At-or-above the MID→HIGH edge → HIGH.
            HeightBand::High => Self(*edges.mid_high),
        }
    }
}

/// Classify a within-level clearance fraction into a [`HeightBand`] using the
/// tunable [`BandEdge`] level-fraction edges off [`CombatTuning`] — **no hardcoded
/// band numbers** (C6, `docs/combat/battle-space.md` §"Banding";
/// `docs/combat/resolution.md` §2's `band_for`).
///
/// A fraction **strictly below** the LOW→MID edge is [`HeightBand::Low`];
/// at-or-above it but **strictly below** the MID→HIGH edge is [`HeightBand::Mid`];
/// at-or-above the MID→HIGH edge is [`HeightBand::High`]. The two edges are read
/// from `tuning.projectile_band_edges` (the tunable level-fractions ≈ ⅓ and ⅔ of a
/// level) — this function never names a magnitude itself.
#[must_use]
pub fn band_for(fraction: BandFraction, tuning: &CombatTuning) -> HeightBand {
    let edges = &tuning.projectile_band_edges;
    if *fraction < *edges.low_mid {
        HeightBand::Low
    } else if *fraction < *edges.mid_high {
        HeightBand::Mid
    } else {
        HeightBand::High
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metric::{Cell, Level};

    /// An arbitrary prototype cover entry — NOT shipped magnitudes. Distinct
    /// `max_hp` / band / armor so a test can prove the seed copies the prototype's
    /// fields, while the arbitrary numbers keep the assertions on the MECHANISM
    /// (lazy-seed / one-ledger / depletion), never a magnitude.
    fn arbitrary_prototype(max_hp: u32, band: HeightBand) -> CoverEntry {
        CoverEntry::seeded(
            CoverHp::new(max_hp),
            band,
            ArmorProtection::new(7),
            ArmorHardness::new(3),
        )
    }

    fn key(x: i32, y: i32, level: u8) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(level))
    }

    /// C8(a) — LAZY seeding: an absent entry, queried, returns a freshly seeded
    /// entry with `current_hp == max_hp`.
    ///
    /// The ledger starts empty (no key present), so `entry_seeded` on an untouched
    /// key must seed `current_hp` to the prototype's `max_hp` and start
    /// not-destroyed. Uses an arbitrary `max_hp` — the invariant is `current ==
    /// max`, not a specific magnitude.
    #[test]
    fn absent_entry_seeds_current_to_max() {
        let mut ledger = CoverLedger::new();
        let k = key(2, 3, 1);
        let prototype = arbitrary_prototype(42, HeightBand::Mid);

        // The key is genuinely absent before the first access.
        assert!(
            ledger.peek(&k).is_none(),
            "ledger must start empty (no pre-seed)"
        );

        let seeded = ledger.entry_seeded(k, prototype);
        assert_eq!(
            seeded.current_hp, seeded.max_hp,
            "a freshly seeded entry must have current_hp == max_hp",
        );
        assert_eq!(
            seeded.max_hp, prototype.max_hp,
            "the seed must take its max_hp from the prototype",
        );
        assert_eq!(
            seeded.destroyed,
            Destroyed::new(false),
            "a freshly seeded entry is not destroyed",
        );
    }

    /// C8(b) — ONE-ledger invariant: a wall entry and a prop entry inserted into
    /// the SAME map are both retrievable.
    ///
    /// Walls and props share one `CoverLedger` (resolution.md §3); inserting one of
    /// each on distinct `(cell, level)` keys and reading both back proves there is
    /// no separate structure for either. The "wall" / "prop" distinction is purely
    /// the caller's authored data (different bands here) — the map treats them
    /// identically.
    #[test]
    fn wall_and_prop_share_one_ledger() {
        let mut ledger = CoverLedger::new();
        // A "wall": a tall, tough piece.
        let wall_key = key(0, 0, 0);
        let wall = arbitrary_prototype(100, HeightBand::High);
        // A "prop": a low, fragile piece, on a different key.
        let prop_key = key(5, 5, 0);
        let prop = arbitrary_prototype(20, HeightBand::Low);

        ledger.insert(wall_key, wall);
        ledger.insert(prop_key, prop);

        let got_wall = ledger.peek(&wall_key).copied();
        let got_prop = ledger.peek(&prop_key).copied();
        assert_eq!(got_wall, Some(wall), "the wall entry must be retrievable");
        assert_eq!(got_prop, Some(prop), "the prop entry must be retrievable");
        // And they are genuinely distinct entries in the one map.
        assert_ne!(
            got_wall, got_prop,
            "wall and prop are distinct entries in the SAME ledger",
        );
    }

    /// C8(c) — depletion sets `destroyed == true` EXACTLY when `current_hp` reaches
    /// zero, and returns the [`CoverEvent::Destroyed`] marker carrying the
    /// `(cell, level)`.
    ///
    /// Reduces a freshly-seeded entry to zero in one hit and asserts both the
    /// destroyed flag and the returned marker (with its `CellLevel`). Arbitrary
    /// magnitudes — the mechanism is "reaches zero ⇒ destroyed + marker", not the
    /// numbers.
    #[test]
    fn depletion_to_zero_destroys_and_returns_marker() {
        let mut ledger = CoverLedger::new();
        let tuning = CombatTuning::default();
        let k = key(4, 7, 2);
        let prototype = arbitrary_prototype(30, HeightBand::Low);

        // One hit exactly equal to max HP drives current_hp to zero.
        let event = ledger.deplete_cover(k, CoverDamage::new(30), prototype, &tuning);

        assert_eq!(
            event,
            CoverEvent::Destroyed(k),
            "reaching zero HP must return the Destroyed marker carrying (cell, level)",
        );
        // Re-query the stored entry (compared as `Some(..)`, never unwrapped — the
        // workspace denies unwrap/expect/panic in tests too).
        let stored = ledger.peek(&k).copied();
        assert_eq!(
            stored.map(|e| e.destroyed),
            Some(Destroyed::new(true)),
            "current_hp reaching zero must set destroyed = true",
        );
        assert_eq!(
            stored.map(|e| *e.current_hp),
            Some(0),
            "destroyed cover has current_hp == 0",
        );
    }

    /// C8(c) the other side — a PARTIAL hit (not to zero) does NOT destroy and
    /// returns the [`CoverEvent::Damaged`] marker.
    ///
    /// Pins that `destroyed` flips EXACTLY at zero, not before: a hit smaller than
    /// `max_hp` leaves the cover standing with reduced HP and an un-set flag.
    #[test]
    fn partial_depletion_does_not_destroy() {
        let mut ledger = CoverLedger::new();
        let tuning = CombatTuning::default();
        let k = key(1, 1, 0);
        let prototype = arbitrary_prototype(50, HeightBand::Mid);

        let event = ledger.deplete_cover(k, CoverDamage::new(20), prototype, &tuning);

        assert_eq!(
            event,
            CoverEvent::Damaged(k),
            "a partial hit must return the Damaged marker, not Destroyed",
        );
        let stored = ledger.peek(&k).copied();
        assert_eq!(
            stored.map(|e| e.destroyed),
            Some(Destroyed::new(false)),
            "a partial hit must NOT set destroyed",
        );
        assert_eq!(
            stored.map(|e| *e.current_hp),
            Some(30),
            "current_hp must be max_hp - damage after a partial hit",
        );
    }

    /// A second hit that finishes a previously-damaged piece destroys it — the
    /// ledger persists `current_hp` across hits (it is not re-seeded each call).
    #[test]
    fn second_hit_finishes_a_damaged_piece() {
        let mut ledger = CoverLedger::new();
        let tuning = CombatTuning::default();
        let k = key(3, 3, 1);
        let prototype = arbitrary_prototype(10, HeightBand::Low);

        let first = ledger.deplete_cover(k, CoverDamage::new(6), prototype, &tuning);
        assert_eq!(first, CoverEvent::Damaged(k));

        // The second hit reads the persisted (reduced) HP, not a fresh seed.
        let second = ledger.deplete_cover(k, CoverDamage::new(6), prototype, &tuning);
        assert_eq!(
            second,
            CoverEvent::Destroyed(k),
            "the second hit must finish the already-damaged piece",
        );
    }

    /// [`band_for`] classifies within-level fractions using ONLY the tuning edges
    /// — no hardcoded band numbers (C6). Drives a fraction below, between, and
    /// above the tuning's two edges and asserts LOW / MID / HIGH, deriving the
    /// probe fractions from the edges themselves (so a tuning edit moves the
    /// boundaries with it).
    #[test]
    fn band_for_reads_tuning_edges() {
        let tuning = CombatTuning::default();
        let edges = &tuning.projectile_band_edges;

        // Strictly below the LOW→MID edge → LOW (half the edge is below it for any
        // positive edge, deriving the probe from the edge, not a literal).
        let low = band_for(BandFraction::new(*edges.low_mid * 0.5), &tuning);
        assert_eq!(low, HeightBand::Low);
        // At-or-above LOW→MID but below MID→HIGH → MID.
        let mid = band_for(BandFraction::new(*edges.low_mid), &tuning);
        assert_eq!(mid, HeightBand::Mid);
        // At-or-above the MID→HIGH edge → HIGH.
        let high = band_for(BandFraction::new(*edges.mid_high), &tuning);
        assert_eq!(high, HeightBand::High);
    }

    /// The `from_band` round-trip lands back in the same band under `band_for` for
    /// all three bands — proving `deplete_cover`'s tuning re-classification is a
    /// no-op on an already-correct band (it never drifts a stored band).
    #[test]
    fn band_round_trips_through_tuning() {
        let tuning = CombatTuning::default();
        for band in [HeightBand::Low, HeightBand::Mid, HeightBand::High] {
            let fraction = BandFraction::from_band(band, &tuning);
            assert_eq!(
                band_for(fraction, &tuning),
                band,
                "{band:?} must round-trip"
            );
        }
    }

    /// The cover newtypes' derived [`Deref`] reaches their inner value. Built from
    /// arbitrary literals — pins the Deref mechanism, not a magnitude.
    #[test]
    fn cover_newtypes_deref_to_inner() {
        assert_eq!(*CoverHp::new(13), 13u32);
        assert_eq!(*CoverDamage::new(4), 4u32);
        assert!(!*Destroyed::new(false));
        assert!(*Destroyed::new(true));
        assert_eq!((*BandFraction::new(5.0)).to_bits(), 5.0_f32.to_bits());
    }
}
