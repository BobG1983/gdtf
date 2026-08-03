use crate::{
    armor::ArmorPiece,
    matchup::{Matchup, MatchupMultiplier, matchup_multiplier},
    resolve_hit::result::{
        DamageMagnitude, DamageReal, HitResult, HpDamage, IntegrityWear, PenetratingDamage,
    },
    tuning::CombatTuning,
    weapon::{WeaponDamage, WeaponPunch, WeaponShred},
};

/// localized `#[expect]` is the crate's guarded-cast idiom (see
const fn round_to_i32(value: DamageReal) -> DamageMagnitude {
    let rounded = value.get().round();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to the i32 range below, so the cast cannot wrap; fractional part is gone after round"
    )]
    let clamped = rounded.clamp(i32::MIN as f32, i32::MAX as f32) as i32;
    DamageMagnitude::new(clamped)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "i32 stat → f32 for the matchup multiply; weapon punch/shred magnitudes are far inside f32's exact-integer range"
)]
pub(super) fn scale_by_matchup(stat: DamageMagnitude, mult: MatchupMultiplier) -> DamageMagnitude {
    round_to_i32(DamageReal::new(*stat as f32 * *mult))
}

#[must_use]
pub fn resolve_hit(
    weapon_damage: WeaponDamage,
    weapon_punch: WeaponPunch,
    weapon_shred: WeaponShred,
    armor: &ArmorPiece,
    matchup: Matchup,
    tuning: &CombatTuning,
) -> HitResult {
    let mult = matchup_multiplier(matchup, tuning);

    let punch_scaled = scale_by_matchup(DamageMagnitude::new(*weapon_punch), mult);
    let shred_scaled = scale_by_matchup(DamageMagnitude::new(*weapon_shred), mult);

    let floor = *armor.floor;
    let protection = *armor.protection;
    let hardness = *armor.hardness;
    let damage = *weapon_damage;

    let eff_pen = (*punch_scaled - hardness).max(0);

    let inner = damage - (protection - eff_pen).max(0);

    let hp_damage = HpDamage::new(floor.max(inner));

    let penetrating = PenetratingDamage::new(inner.max(0));

    let wear = IntegrityWear::new(protection.min(damage) + eff_pen + *shred_scaled);

    HitResult {
        penetrating,
        hp_damage,
        wear,
    }
}
