//! The falls tuning leaf (GTW-523) — [`PerStoreyDamage`], the per-storey fall-damage
//! magnitude a slab-destroy fall deals.
//!
//! `docs/combat/resolution.md` §Falls / `docs/combat/combat.md` (fall damage is now
//! in-scope): a ganger standing on a slab that is destroyed under it **falls**, and the
//! blow it takes is **linear in the storeys fallen** — `magnitude = per_storey_damage ×
//! storeys_fallen`. The equation FORM lives in code (the falls system); this leaf is the
//! only balance number it reads.

use bevy::prelude::Deref;
use serde::Deserialize;

/// The **per-storey fall damage** — the base damage magnitude a fall deals **per storey
/// fallen** (`docs/combat/resolution.md` §Falls; GTW-523 C4).
///
/// The falls damage is **weight-free and LINEAR**: a fall of `n` storeys deals
/// `per_storey_damage × n` raw damage, routed as a [`Matchup::Neutral`](crate::matchup::Matchup::Neutral)
/// kinetic hit through the EXISTING `resolve_hit` → `roll_severity` → `apply_hit` pipeline
/// (armor is honored; only the ganger's weight is deferred — GTW-452 owns weighting). An
/// `i32` to feed the signed per-hit damage formula directly (the same signed reasoning the
/// weapon `damage` term uses). The default is a **starting point**, tunable balance data —
/// tests assert only the FORMULA / relations (monotone in storeys, a hot-edit shifts the
/// blow), never this magnitude. `#[serde(transparent)]` lets it parse a bare RON scalar;
/// private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct PerStoreyDamage(i32);

impl PerStoreyDamage {
    /// Build a per-storey fall-damage magnitude from its base damage (a starting point,
    /// TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `i32` private (house style)
    /// while letting the falls tests and any programmatic tuning edit build a value
    /// without a bare `i32` escaping; shipped values come from the `.ron` via the derived
    /// [`Deserialize`].
    #[must_use]
    pub const fn new(damage: i32) -> Self {
        Self(damage)
    }
}

impl Default for PerStoreyDamage {
    fn default() -> Self {
        // 6 base damage per storey fallen — a STARTING POINT (tunable balance data): a
        // one-storey drop is a solid blow, a multi-storey plunge scales linearly and can
        // kill. Value-agnostic tests only (monotone in storeys, hot-edit shifts damage),
        // never a pinned magnitude.
        Self(6)
    }
}
