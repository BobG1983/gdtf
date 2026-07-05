//! The shared faced-cover fixture, reached via `use super::support::*;`.

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, HeightBand},
};

/// An arbitrary faced-cover entry at `band` — NOT shipped magnitudes; the
/// stability layer only reads `height_band`, so the HP/armor are filler.
pub(super) fn faced_cover(band: HeightBand) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        band,
        ArmorProtection::new(1),
        ArmorHardness::new(1),
    )
}
