//! Accrue damage to open ground.

use crate::{
    metric::{Cell, CellLevel},
    resolve_and_apply::report::HitVerdict,
    surface::GroundDamage,
    weapon::WeaponStats,
};

/// Ground damage recorded at a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroundAccrual {
    /// Cell that was hit.
    pub cell: Cell,
    /// Damage amount.
    pub amount: GroundDamage,
}

impl GroundAccrual {
    /// Build a ground accrual record.
    #[must_use]
    pub const fn new(cell: Cell, amount: GroundDamage) -> Self {
        Self { cell, amount }
    }
}

/// Record ground damage from a weapon hit.
pub(in crate::damage_resolution::resolve_and_apply) fn fold(
    at: CellLevel,
    weapon: WeaponStats<'_>,
) -> HitVerdict {
    let cell = at.cell();
    let amount = GroundDamage::new(u32::try_from((**weapon.damage).max(0)).unwrap_or(0));
    HitVerdict::Ground(GroundAccrual::new(cell, amount))
}
