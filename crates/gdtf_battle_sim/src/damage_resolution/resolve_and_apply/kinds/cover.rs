//! Apply a shot that struck cover.

use crate::{
    armor::{ArmorFloor, ArmorIntegrity, ArmorPiece, ArmorType},
    cover::{CoverDamage, CoverEntry, CoverEvent, CoverLedger},
    matchup::Matchup,
    metric::CellLevel,
    resolve_and_apply::report::HitVerdict,
    resolve_hit::{HpDamage, resolve_hit},
    tuning::CombatTuning,
    weapon::WeaponStats,
};

/// Result of damaging cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverVerdict {
    /// Cell whose cover was destroyed, if any.
    pub destroyed: Option<CellLevel>,
}

/// Build a temporary armor piece from a cover entry.
pub(crate) const fn cover_armor_piece(entry: &CoverEntry) -> ArmorPiece {
    ArmorPiece::new(
        ArmorFloor::new(0),
        entry.armor_protection,
        ArmorIntegrity::new(0),
        entry.armor_hardness,
        ArmorType::DEFAULT,
    )
}

/// Convert HP damage into cover damage.
pub(crate) fn cover_damage_from_hp(hp_damage: HpDamage) -> CoverDamage {
    CoverDamage::new(u32::try_from((*hp_damage).max(0)).unwrap_or(0))
}

/// Resolve and apply damage to cover at a cell.
pub(in crate::damage_resolution::resolve_and_apply) fn fold(
    entry: &CoverEntry,
    at: CellLevel,
    weapon: WeaponStats<'_>,
    cover: &mut CoverLedger,
    tuning: &CombatTuning,
) -> HitVerdict {
    let piece = cover_armor_piece(entry);
    let hit = resolve_hit(
        *weapon.damage,
        *weapon.punch,
        *weapon.shred,
        &piece,
        Matchup::Neutral,
        tuning,
    );

    let removed = cover_damage_from_hp(hit.hp_damage);

    let destroyed = match cover.deplete_cover(at, removed, *entry, tuning) {
        CoverEvent::Destroyed(cell) => Some(cell),
        CoverEvent::Damaged(_) => None,
    };
    HitVerdict::Cover(CoverVerdict { destroyed })
}
