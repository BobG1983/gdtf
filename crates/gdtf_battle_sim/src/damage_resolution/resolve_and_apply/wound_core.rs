use bevy::prelude::Entity;

use crate::{
    apply_hit::{GangerHitTarget, apply_hit},
    armor::{ArmorPiece, BodyPart},
    armor_wear::ArmorWearOutcome,
    ganger::{LifeState, Luck},
    injuries::{DamageContext, InjuryRegistry, InjuryTables, RolledInjury, roll_injury},
    matchup::Matchup,
    melee::{MeleeDamageMult, apply_melee_multiplier},
    resolve_and_apply::report::TargetGanger,
    resolve_hit::{HitResult, resolve_hit},
    rng::{InjuryRng, SeverityRng},
    severity::{Severity, SeverityInputs, part_severity_mod, roll_severity},
    tuning::CombatTuning,
    weapon::{FatalBias, WeaponDamage, WeaponPunch, WeaponShred},
};

pub(crate) struct WoundBlow {
        pub part:         BodyPart,
        pub damage:       WeaponDamage,
        pub punch:        WeaponPunch,
        pub shred:        WeaponShred,
        pub piece:        ArmorPiece,
            pub matchup:      Matchup,
        pub fatal_bias:   FatalBias,
        pub shooter_luck: Luck,
                    pub context:      DamageContext,
                                    pub damage_mult:  Option<MeleeDamageMult>,
}

pub(crate) struct WoundCoreInputs<'a> {
        pub blow:          WoundBlow,
        pub target:        TargetGanger<'a>,
        pub target_entity: Entity,
        pub tuning:        &'a CombatTuning,
        pub severity_rng:  &'a mut SeverityRng,
        pub tables:        &'a InjuryTables,
        pub registry:      &'a InjuryRegistry,
            pub injury_rng:    &'a mut InjuryRng,
}

pub(crate) struct WoundSynthesis {
            pub matchup:    Matchup,
        pub hit:        HitResult,
        pub severity:   Severity,
        pub wear:       ArmorWearOutcome,
        pub life_after: LifeState,
                pub injury:     Option<RolledInjury>,
}

#[must_use]
pub(crate) fn synthesize_wound(inputs: WoundCoreInputs<'_>) -> Option<WoundSynthesis> {
    let WoundCoreInputs {
        blow,
        target,
        target_entity,
        tuning,
        severity_rng,
        tables,
        registry,
        injury_rng,
    } = inputs;

    if *target.life == LifeState::Dead {
        return None;
    }

    let resolved = resolve_hit(
        blow.damage,
        blow.punch,
        blow.shred,
        &blow.piece,
        blow.matchup,
        tuning,
    );

    let hit = match blow.damage_mult {
        Some(mult) => apply_melee_multiplier(resolved, mult),
        None => resolved,
    };

    let severity = roll_severity(
        &SeverityInputs::new(
            hit.penetrating,
            target.toughness,
            part_severity_mod(blow.part),
            blow.fatal_bias,
            blow.shooter_luck,
            target.luck,
        ),
        &tuning.severity_scaling,
        severity_rng,
    );

    let wear = apply_hit(
        GangerHitTarget {
            hp:        target.hp,
            wounds:    target.wounds,
            life:      target.life,
            integrity: target.piece.map(|p| p.integrity),
            inflicted: target.inflicted,
        },
        &hit,
        severity,
        blow.part,
        target_entity,
        tuning,
    );

    let injury = roll_injury(
        blow.part,
        severity,
        blow.context,
        tables,
        registry,
        injury_rng,
    );

    Some(WoundSynthesis {
        matchup: blow.matchup,
        hit,
        severity,
        wear,
        life_after: *target.life,
        injury,
    })
}
