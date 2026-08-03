use crate::{
    metric::{Cell, CellLevel},
    resolve_and_apply::report::HitVerdict,
    surface::GroundDamage,
    weapon::WeaponStats,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroundAccrual {
        pub cell:   Cell,
        pub amount: GroundDamage,
}

impl GroundAccrual {
            #[must_use]
    pub const fn new(cell: Cell, amount: GroundDamage) -> Self {
        Self { cell, amount }
    }
}

pub(in crate::damage_resolution::resolve_and_apply) fn fold(
    at: CellLevel,
    weapon: WeaponStats<'_>,
) -> HitVerdict {
    let cell = at.cell();
    let amount = GroundDamage::new(u32::try_from((**weapon.damage).max(0)).unwrap_or(0));
    HitVerdict::Ground(GroundAccrual::new(cell, amount))
}
