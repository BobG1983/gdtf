use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, HeightBand},
};

pub(super) fn faced_cover(band: HeightBand) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        band,
        ArmorProtection::new(1),
        ArmorHardness::new(1),
    )
}
