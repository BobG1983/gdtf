//! Inputs for [`resolve_melee_strike`](super::resolve_melee_strike).

use crate::{
    ganger::{Fight, Luck},
    injuries::{InjuryRegistry, InjuryTables},
    rng::{FightRng, InjuryRng, SeverityRng, ShotRng},
    tuning::CombatTuning,
    weapon::{DamageType, FatalBias, WeaponDamage, WeaponPunch, WeaponShred},
};

/// Weapon stats used for a melee blow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeleeWeaponHit<'a> {
    /// Base damage.
    pub damage: &'a WeaponDamage,
    /// Punch.
    pub punch: &'a WeaponPunch,
    /// Shred.
    pub shred: &'a WeaponShred,
    /// Damage type for armor matchup.
    pub damage_type: &'a DamageType,
    /// Fatal bias.
    pub fatal_bias: &'a FatalBias,
}

/// Attacker and defender fight stats (and attacker luck).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Combatants {
    /// Attacker fight value.
    pub attacker_fight: Fight,
    /// Defender fight value.
    pub defender_fight: Fight,
    /// Attacker luck.
    pub attacker_luck: Luck,
}

/// Shared tables and RNGs for a melee strike.
pub struct MeleeStrikeEnv<'a> {
    /// Combat tuning.
    pub tuning: &'a CombatTuning,
    /// Injury tables.
    pub tables: &'a InjuryTables,
    /// Injury registry.
    pub registry: &'a InjuryRegistry,
    /// Fight stream.
    pub fight_rng: &'a mut FightRng,
    /// Body-part / shot stream.
    pub shot_rng: &'a mut ShotRng,
    /// Severity stream.
    pub severity_rng: &'a mut SeverityRng,
    /// Injury stream.
    pub injury_rng: &'a mut InjuryRng,
}
