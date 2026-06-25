//! The GTW-365 slab-defaults tuning leaf — the uniform HP + armor a floor/roof slab
//! lazily seeds to (`docs/combat/resolution.md` §3.1; user-ruled 2026-06-22).
//!
//! Slabs are **uniform level structure**: a situation authors them as a bare
//! `(cell, level)` list with NO per-slab HP (unlike cover, whose `CoverSpawn` authors
//! per-piece HP/armor). So a slab's structural HP + armor live here as a combat-tuning
//! leaf, **genuinely consumed** by the [`SlabLedger`](crate::slab::SlabLedger)'s
//! lazy-seed (via `SlabLedger::prototype_for`) on the live depletion path — the C7
//! "no dead leaf" requirement. The magnitudes are TBD tuning; tests assert only the
//! mechanism / parse, never a shipped magnitude.

use bevy::prelude::Deref;
use serde::Deserialize;

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    slab::SlabHp,
};

/// A floor/roof slab's default structural HP — the `max_hp` a slab lazily seeds to in
/// the [`SlabLedger`](crate::slab::SlabLedger) when it is first struck.
///
/// A distinct newtype over `u16` (no-bare-types: a slab default HP is a domain value,
/// not a bare integer); the ledger converts it to the wider [`SlabHp`] `u32` pool at
/// the seed site. Private inner + derived [`Deref`]; `#[serde(transparent)]` lets it
/// parse a bare RON scalar (the tuning-leaf precedent). **Tunable** balance data —
/// tests assert only the parse / mechanism, never the magnitude.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct SlabDefaultHp(u16);

impl SlabDefaultHp {
    /// Build a slab default-HP value from its magnitude (tunable balance data).
    #[must_use]
    pub const fn new(hp: u16) -> Self {
        Self(hp)
    }
}

impl Default for SlabDefaultHp {
    fn default() -> Self {
        // A value-agnostic STARTING POINT (tunable): high enough that one round of a
        // typical weapon damages but does not one-shot a slab, so a slab depletes over
        // MULTIPLE strikes (C4). Tests assert only the mechanism, never this magnitude.
        Self(120)
    }
}

/// The **slab-defaults table** — the uniform HP + armor every floor/roof slab lazily
/// seeds to (the C7 combat-tuning seam, genuinely consumed by
/// [`SlabLedger::prototype_for`](crate::slab::SlabLedger::prototype_for)).
///
/// One field per stat: the structural [`hp`](SlabDefaults::hp), and the slab's own
/// armor [`armor_protection`](SlabDefaults::armor_protection) /
/// [`armor_hardness`](SlabDefaults::armor_hardness) (**reusing** the GTW-153 armor
/// newtypes — a slab uses the same armor model as a ganger and cover, so the cover-hit
/// damage formula resolves a slab hit the same way). Uniform across the whole level —
/// slabs carry no per-piece authored data, so this is the sole HP/armor source for a
/// struck slab. The accessors return the ledger-facing types (`SlabHp` / the armor
/// newtypes) so the seam reads the same shape the cover prototype does. The magnitudes
/// are **tunable** balance data; tests assert only the mechanism / parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct SlabDefaults {
    /// The structural HP a slab seeds to (its `max_hp`).
    pub default_hp:               SlabDefaultHp,
    /// The slab's damage-reduction stat (same armor model as a ganger / cover).
    pub default_armor_protection: ArmorProtection,
    /// The penetration the slab shrugs off (same armor model as a ganger / cover).
    pub default_armor_hardness:   ArmorHardness,
}

impl SlabDefaults {
    /// The slab's seed HP as a ledger-facing [`SlabHp`] — widens the tunable `u16` to
    /// the `u32` pool the ledger keeps.
    #[must_use]
    pub fn hp(&self) -> SlabHp {
        SlabHp::new(u32::from(*self.default_hp))
    }

    /// The slab's seed [`ArmorProtection`] — the soak the cover-hit damage formula
    /// resolves a slab hit against.
    #[must_use]
    pub const fn armor_protection(&self) -> ArmorProtection {
        self.default_armor_protection
    }

    /// The slab's seed [`ArmorHardness`] — the penetration the slab shrugs off.
    #[must_use]
    pub const fn armor_hardness(&self) -> ArmorHardness {
        self.default_armor_hardness
    }
}

impl SlabDefaults {
    /// The code-only no-panic fallback for an unauthored slab strike — used by the
    /// damage-resolution fold when a slab is struck that has no authored ledger entry
    /// (an out-of-bounds or non-authored cell). NOT the removed `tuning.ron
    /// slab_defaults` leaf (GTW-396 removed that field); the fallback fires ONLY when
    /// `SlabLedger::entry_seeded`'s `.or_insert` path is taken (i.e., the slab was
    /// never eagerly-inserted at setup). Authored slabs are always pre-seeded at setup
    /// (GTW-396 Decision C eager-seed), so this constant is the "unauthored strike"
    /// backstop.
    ///
    /// `const` so the fold site can declare it as a block-local `const` (no allocation,
    /// no `Res<_>` read — zero overhead on the hot depletion path).
    pub const FALLBACK: Self = Self {
        default_hp:               SlabDefaultHp::new(120),
        default_armor_protection: ArmorProtection::new(4),
        default_armor_hardness:   ArmorHardness::new(2),
    };
}

impl Default for SlabDefaults {
    fn default() -> Self {
        // Value-agnostic STARTING POINTS — mirror the FALLBACK const (so `default()`
        // and `FALLBACK` agree). Tests assert only the depletion/destruction mechanism,
        // never these magnitudes (brittle-test rule).
        Self::FALLBACK
    }
}
