//! The cover value types — the structural-HP / damage / destroyed newtypes, the
//! [`HeightBand`] clearance enum, the per-`(cell, level)` [`CoverEntry`] record, and
//! the [`CoverEvent`] depletion outcome. Every field is a named newtype
//! (no-bare-types).

use bevy::prelude::Deref;
use serde::Deserialize;

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    metric::CellLevel,
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
/// `#[serde(transparent)]` lets an authored cover-HP parse as a bare integer.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct CoverHp(u32);

impl CoverHp {
    /// Build a cover-HP value from its magnitude (per-object data, TBD tuning).
    #[must_use]
    pub const fn new(hp: u32) -> Self {
        Self(hp)
    }

    /// Reduce this HP by `damage`, **saturating at zero** — cover HP is a
    /// non-negative pool and depletion can never carry it below zero (at zero the
    /// cover is destroyed).
    #[must_use]
    pub fn saturating_sub(self, damage: CoverDamage) -> Self {
        Self(self.0.saturating_sub(*damage))
    }
}

/// A piece of cover's height band — which clearance band it occupies, so the
/// march knows what a round must fly strictly higher than to clear it
/// (`docs/combat/resolution.md` §3: cover "occupies its cell at its `cover_height`
/// band (LOW / MID / HIGH)").
///
/// A named domain enum (the §2/§3 LOW/MID/HIGH banding), introduced here at first
/// use — not a bare index. The level-fraction → band classification lives in
/// [`band_for`](crate::cover::band_for), which reads the tunable
/// [`BandEdge`](crate::tuning::BandEdge) level-fraction edges; this enum never
/// carries a fraction itself.
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
/// not a bare boolean). Set `true` by [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover)
/// exactly when `current_hp` reaches zero; the occupancy/prop consequences of that
/// are GTW-35. Private inner + derived [`Deref`] (house style).
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

/// The outcome of spending cover HP — what
/// [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover) returns.
///
/// A named domain enum (not a bare `Option`/`bool`): either the hit only
/// **damaged** the cover, or it **destroyed** it, in which case the variant carries
/// the [`CellLevel`] for the presenter's reactions (`docs/combat/resolution.md`
/// §3: "depletion emits a cover-destroyed event carrying (cell, level)"; the
/// model's `CoverDestroyed { cell, level }` signal, a model-authoritative fact the
/// view mirrors — ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`). This slice
/// emits the marker only — acting on it (occupancy + prop removal) is GTW-35.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoverEvent {
    /// The cover took damage but still stands (`current_hp > 0`).
    Damaged(CellLevel),
    /// The cover was destroyed this hit (`current_hp` reached zero) — carries the
    /// `(cell, level)` the destroyed cover occupied.
    Destroyed(CellLevel),
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
