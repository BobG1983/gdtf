//! Apply a shot that struck a floor slab.

use crate::{
    armor::{ArmorFloor, ArmorIntegrity, ArmorPiece, ArmorType},
    matchup::Matchup,
    metric::CellLevel,
    resolve_and_apply::report::HitVerdict,
    resolve_hit::resolve_hit,
    slab::{SlabDamage, SlabEntry, SlabEvent, SlabLedger},
    tuning::CombatTuning,
    weapon::WeaponStats,
};

/// Result of damaging a slab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabVerdict {
    /// Cell whose slab was destroyed, if any.
    pub destroyed: Option<CellLevel>,
}

const fn slab_armor_piece(entry: &SlabEntry) -> ArmorPiece {
    ArmorPiece::new(
        ArmorFloor::new(0),
        entry.armor_protection,
        ArmorIntegrity::new(0),
        entry.armor_hardness,
        ArmorType::DEFAULT,
    )
}

/// Resolve and apply damage to a slab at a cell.
pub(in crate::damage_resolution::resolve_and_apply) fn fold(
    at: CellLevel,
    weapon: WeaponStats<'_>,
    slab: &mut SlabLedger,
    tuning: &CombatTuning,
) -> HitVerdict {
    const SLAB_FALLBACK_DEFAULTS: crate::tuning::SlabDefaults =
        crate::tuning::SlabDefaults::FALLBACK;
    let prototype = SlabLedger::prototype_for(at, &SLAB_FALLBACK_DEFAULTS);

    let piece = slab_armor_piece(&prototype);
    let hit = resolve_hit(
        *weapon.damage,
        *weapon.punch,
        *weapon.shred,
        &piece,
        Matchup::Neutral,
        tuning,
    );

    let removed = SlabDamage::new(u32::try_from((*hit.hp_damage).max(0)).unwrap_or(0));

    let destroyed = match slab.deplete_slab(at, removed, prototype) {
        SlabEvent::Destroyed(cell) => Some(cell),
        SlabEvent::Damaged(_) => None,
    };
    HitVerdict::Slab(SlabVerdict { destroyed })
}
