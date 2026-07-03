//! The **ground** struck-kind — the GTW-366 ground-accrual arm of the E3.9 fold
//! (`docs/combat/resolution.md` §3.2; user-ruled 2026-06-22) and its per-kind verdict
//! payload ([`GroundAccrual`]), owned here per the GTW-573 one-module-per-kind layout —
//! the ground-accrual mirror of [`cover`](super::cover) / [`slab`](super::slab).
//!
//! The ground is **damaged, never destroyed**, so there is NO armor / HP / depletion
//! math here — the accrued amount is simply the round's `weapon_damage` (GTW-366 C4,
//! NOT a constant and NOT a tuning leaf — the ground has no HP/armor defaults),
//! recorded against the ground-plane [`Cell`] the round exited through. Mutates
//! NOTHING (no ledger, no grid, no ganger) and takes **no** RNG draw on either stream
//! — purely a frozen verdict (crater FX is a later ticket).

use crate::{
    metric::{Cell, CellLevel},
    resolve_and_apply::report::HitVerdict,
    surface::GroundDamage,
    weapon::WeaponStats,
};

/// The **ground-accrual verdict** — the [`Cell`] a round struck the ground at and the
/// [`GroundDamage`] it dealt there (GTW-366, `docs/combat/resolution.md` §3.2).
///
/// The GTW-573 per-kind payload of [`HitVerdict::Ground`]: a frozen `Copy` record of
/// two named domain newtypes (no bare primitive, no pixel) — the ground-plane [`Cell`]
/// the round exited the bottom of the voxel column at, and the round's
/// [`GroundDamage`] (its `weapon_damage`, NOT a constant — GTW-366 C4). The fire
/// path's [`dispatch_fire`](crate::acts::dispatch_fire) bridges it into a buffered
/// [`GroundAccrued`](crate::occupancy_sync::GroundAccrued) message, which
/// [`sync_accrued_ground`](crate::occupancy_sync::sync_accrued_ground) ACCRUES
/// (monotonically) onto the [`SurfaceGrid`](crate::surface::SurfaceGrid). The ground
/// is **damaged, never destroyed**, so this records accrual, never destruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroundAccrual {
    /// The ground-plane [`Cell`] the round struck — the accumulator key.
    pub cell:   Cell,
    /// The [`GroundDamage`] the round dealt — its `weapon_damage`, accrued onto the cell.
    pub amount: GroundDamage,
}

impl GroundAccrual {
    /// Build a ground-accrual verdict for the `cell` the round struck and the `amount`
    /// of [`GroundDamage`] it dealt (the round's `weapon_damage`).
    #[must_use]
    pub const fn new(cell: Cell, amount: GroundDamage) -> Self {
        Self { cell, amount }
    }
}

/// Fold a **[`ShotKind::Ground`](crate::resolve_coarse::ShotKind::Ground)** outcome —
/// freeze the [`GroundAccrual`] verdict for a round that struck the ground at `at`
/// (`docs/combat/resolution.md` §3.2).
///
/// The ground-plane (x, y) the round exited through is the accumulator key (a
/// [`Cell`], never the storey z — via the canonical
/// [`CellLevel::cell`](crate::metric::CellLevel) accessor, GTW-565). The accrued
/// amount is the round's `weapon_damage`;
/// [`WeaponDamage`](crate::weapon::WeaponDamage) is a signed `i32` (it carries a
/// `Default` spawn sentinel that can read `0`/negative before the real value seeds),
/// so a negative value clamps to zero, mirroring the cover / slab
/// `u32::try_from(... .max(0))` conversion (ground damage is a non-negative pool).
/// Mutates NOTHING and takes **no** RNG draw (deterministic, replay-safe — pinned by
/// `test::draw_discipline`).
pub(in crate::damage_resolution::resolve_and_apply) fn fold(
    at: CellLevel,
    weapon: WeaponStats<'_>,
) -> HitVerdict {
    let cell = at.cell();
    // C4: the accrued amount is the round's weapon_damage — NOT a hardcoded constant and
    // NOT a tuning leaf. A negative / sentinel value clamps to zero (ground damage is a
    // non-negative pool), the same conversion the cover / slab HP-loss takes.
    let amount = GroundDamage::new(u32::try_from((**weapon.damage).max(0)).unwrap_or(0));
    HitVerdict::Ground(GroundAccrual::new(cell, amount))
}
