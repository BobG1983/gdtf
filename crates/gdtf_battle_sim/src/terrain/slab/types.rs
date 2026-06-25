//! The slab value types — the structural-HP / damage / destroyed newtypes, the
//! per-`(cell, level)` [`SlabEntry`] record, and the [`SlabEvent`] depletion outcome.
//! Every field is a named newtype (no-bare-types), mirroring the GTW-364 cover types.

use bevy::prelude::Deref;
use serde::Deserialize;

use crate::armor::{ArmorHardness, ArmorProtection};

/// Slab structural HP — the hit points a floor/roof slab carries in the ledger.
///
/// One newtype used for **both** [`SlabEntry::current_hp`] and
/// [`SlabEntry::max_hp`]: they are the same *kind* of value (a structural-HP
/// quantity), distinguished by their field — the [`CoverHp`](crate::cover::CoverHp)
/// precedent for slabs. A `u32` because slab HP is a non-negative pool that depletes
/// to zero (it never tracks below zero — at zero the slab is destroyed). A distinct
/// newtype from `CoverHp` (no-bare-types rule 3: a slab HP pool is not a cover HP
/// pool, even over the same inner `u32`). Private inner + derived [`Deref`] (house
/// style); a magnitude is per-slab data (TBD tuning), not pinned here.
/// `#[serde(transparent)]` lets an authored slab-HP parse as a bare integer (the
/// [`TerrainSpec`](crate::terrain::piece::SlabPieceSpec) authoring path).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct SlabHp(u32);

impl SlabHp {
    /// Build a slab-HP value from its magnitude (per-slab data, TBD tuning).
    #[must_use]
    pub const fn new(hp: u32) -> Self {
        Self(hp)
    }

    /// Reduce this HP by `damage`, **saturating at zero** — slab HP is a
    /// non-negative pool and depletion can never carry it below zero (at zero the
    /// slab is destroyed).
    #[must_use]
    pub fn saturating_sub(self, damage: SlabDamage) -> Self {
        Self(self.0.saturating_sub(*damage))
    }
}

/// Whether a slab has been destroyed — its HP has been spent to zero.
///
/// A named newtype over `bool` (no-bare-types: a destroyed flag is a domain value,
/// not a bare boolean) — the slab mirror of [`cover::Destroyed`](crate::cover::Destroyed).
/// Set `true` by [`SlabLedger::deplete_slab`](crate::slab::SlabLedger::deplete_slab)
/// exactly when `current_hp` reaches zero; the surface-grid / LOS consequences of that
/// are bridged by the fire path (GTW-365). Private inner + derived [`Deref`] (house
/// style).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabDestroyedFlag(bool);

impl SlabDestroyedFlag {
    /// Build a slab-destroyed flag from its boolean state.
    #[must_use]
    pub const fn new(destroyed: bool) -> Self {
        Self(destroyed)
    }
}

/// The slab-HP record for one `(cell, level)` — a floor / roof slab with its **own
/// HP and armor stats**, the same shape the cover ledger keeps for a wall/prop
/// (`docs/combat/resolution.md` §3.1: a slab "has its own **HP and armor stats**,
/// using the same armor/damage model as a ganger and cover"; user-ruled 2026-06-22).
///
/// Every field is a named newtype (no-bare-types): the structural HP pool
/// ([`current_hp`](SlabEntry::current_hp) and [`max_hp`](SlabEntry::max_hp), both
/// [`SlabHp`]); the slab's own armor stats ([`armor_protection`](SlabEntry::armor_protection)
/// and [`armor_hardness`](SlabEntry::armor_hardness), **reusing** the GTW-153 armor
/// newtypes — a slab uses the same armor model as a ganger and cover); and the
/// [`destroyed`](SlabEntry::destroyed) flag. A slab carries **no** height band: unlike
/// cover (which a round must fly strictly higher than to clear), a slab spans the
/// whole z-boundary, so there is no band to clear — the march stops on an intact slab
/// at the boundary regardless of band (`docs/combat/resolution.md` §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabEntry {
    /// The current structural HP remaining — depleted by `deplete_slab`, seeded to
    /// `max_hp` on first access.
    pub current_hp:       SlabHp,
    /// The full structural HP the slab seeds to (its max).
    pub max_hp:           SlabHp,
    /// The slab's damage-reduction stat (same armor model as a ganger / cover).
    pub armor_protection: ArmorProtection,
    /// The penetration this slab shrugs off (same armor model as a ganger / cover).
    pub armor_hardness:   ArmorHardness,
    /// Whether this slab has been destroyed (HP spent to zero).
    pub destroyed:        SlabDestroyedFlag,
}

impl SlabEntry {
    /// Build a slab entry **seeded** with `current_hp == max_hp` and not destroyed —
    /// the freshly-seeded shape returned for a slab on first access (the C4 lazy-seed
    /// shape, mirroring [`CoverEntry::seeded`](crate::cover::CoverEntry::seeded)).
    ///
    /// Takes the slab's `max_hp` and armor stats; the current HP is seeded to the full
    /// `max_hp` and [`SlabDestroyedFlag`] starts `false`.
    #[must_use]
    pub const fn seeded(
        max_hp: SlabHp,
        armor_protection: ArmorProtection,
        armor_hardness: ArmorHardness,
    ) -> Self {
        Self {
            current_hp: max_hp,
            max_hp,
            armor_protection,
            armor_hardness,
            destroyed: SlabDestroyedFlag::new(false),
        }
    }
}

/// The outcome of spending slab HP — what
/// [`SlabLedger::deplete_slab`](crate::slab::SlabLedger::deplete_slab) returns.
///
/// A named domain enum (not a bare `Option`/`bool`) — the slab mirror of
/// [`CoverEvent`](crate::cover::CoverEvent): either the hit only **damaged** the slab,
/// or it **destroyed** it, in which case the variant carries the [`CellLevel`](crate::metric::CellLevel) for the
/// fire path to bridge into a buffered
/// [`SlabDestroyed`](crate::occupancy_sync::SlabDestroyed) message (the
/// fire→deplete→message bridge; a model-authoritative fact the view mirrors — ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlabEvent {
    /// The slab took damage but still stands (`current_hp > 0`).
    Damaged(crate::metric::CellLevel),
    /// The slab was destroyed this hit (`current_hp` reached zero) — carries the
    /// `(cell, level)` the destroyed slab spanned.
    Destroyed(crate::metric::CellLevel),
}

/// A quantity of damage spent against slab HP — the amount one hit removes from a
/// [`SlabEntry`]'s `current_hp`.
///
/// A distinct newtype from [`SlabHp`] (no-bare-types rule 3: distinct concepts get
/// distinct types even over the same inner `u32`) — a damage amount is not an HP pool,
/// and they must not be interchangeable — the [`CoverDamage`](crate::cover::CoverDamage)
/// slab mirror. A magnitude is per-hit data, not pinned here. Private inner + derived
/// [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabDamage(u32);

impl SlabDamage {
    /// Build a slab-damage amount from its magnitude (per-hit data).
    #[must_use]
    pub const fn new(damage: u32) -> Self {
        Self(damage)
    }
}
