//! Melee damage against cover pieces.

use crate::{
    cover::{CoverEntry, CoverEvent, CoverLedger},
    matchup::Matchup,
    melee::{MeleeDamageMult, MeleeWeaponHit, apply_melee_multiplier},
    metric::CellLevel,
    resolve_and_apply::{cover_armor_piece, cover_damage_from_hp},
    resolve_hit::resolve_hit,
    tuning::{CombatTuning, MeleeTuning},
};

/// Structural damage multiplier (from melee max mult).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StructuralMult(f32);

impl StructuralMult {
    /// Use the max melee multiplier from tuning.
    #[must_use]
    pub fn from_tuning(tuning: &MeleeTuning) -> Self {
        Self(*tuning.mult_max)
    }

    /// As a melee damage multiplier.
    #[must_use]
    pub const fn as_melee_mult(self) -> MeleeDamageMult {
        MeleeDamageMult::new(self.0)
    }
}

/// Resolve a melee hit against cover at `at`.
#[must_use]
pub fn resolve_structural_melee(
    weapon: MeleeWeaponHit<'_>,
    entry: &CoverEntry,
    at: CellLevel,
    cover: &mut CoverLedger,
    tuning: &CombatTuning,
) -> CoverEvent {
    let piece = cover_armor_piece(entry);
    let raw_hit = resolve_hit(
        *weapon.damage,
        *weapon.punch,
        *weapon.shred,
        &piece,
        Matchup::Neutral,
        tuning,
    );

    let mult = StructuralMult::from_tuning(&tuning.melee).as_melee_mult();
    let hit = apply_melee_multiplier(raw_hit, mult);

    let removed = cover_damage_from_hp(hit.hp_damage);

    cover.deplete_cover(at, removed, *entry, tuning)
}
