use crate::{
    ganger::{Fight, Luck},
    injuries::{InjuryRegistry, InjuryTables},
    rng::{FightRng, InjuryRng, SeverityRng, ShotRng},
    tuning::CombatTuning,
    weapon::{DamageType, FatalBias, WeaponDamage, WeaponPunch, WeaponShred},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeleeWeaponHit<'a> {
        pub damage:      &'a WeaponDamage,
        pub punch:       &'a WeaponPunch,
        pub shred:       &'a WeaponShred,
        pub damage_type: &'a DamageType,
        pub fatal_bias:  &'a FatalBias,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Combatants {
        pub attacker_fight: Fight,
        pub defender_fight: Fight,
        pub attacker_luck:  Luck,
}

pub struct MeleeStrikeEnv<'a> {
            pub tuning:       &'a CombatTuning,
        pub tables:       &'a InjuryTables,
        pub registry:     &'a InjuryRegistry,
        pub fight_rng:    &'a mut FightRng,
        pub shot_rng:     &'a mut ShotRng,
        pub severity_rng: &'a mut SeverityRng,
            pub injury_rng:   &'a mut InjuryRng,
}
