use bevy::prelude::Entity;

use crate::{
    armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType},
    falls::StoreysFallen,
    ganger::Luck,
    injuries::{DamageContext, InjuryRegistry, InjuryTables, RolledInjury},
    matchup::Matchup,
    resolve_and_apply::{TargetGanger, WoundBlow, WoundCoreInputs, synthesize_wound},
    rng::{InjuryRng, SeverityRng},
    tuning::{CombatTuning, PerStoreyDamage},
    weapon::{FatalBias, WeaponDamage, WeaponPunch, WeaponShred},
};

const BARE_FLESH: ArmorPiece = ArmorPiece::new(
    ArmorFloor::new(0),
    ArmorProtection::new(0),
    ArmorIntegrity::new(0),
    ArmorHardness::new(0),
    ArmorType::DEFAULT,
);

fn fall_damage_magnitude(per_storey: PerStoreyDamage, storeys: StoreysFallen) -> WeaponDamage {
    let magnitude = (*per_storey).saturating_mul(i32::from(*storeys));
    WeaponDamage::new(magnitude)
}

pub(crate) struct FallImpact<'a> {
        pub per_storey:    PerStoreyDamage,
        pub storeys:       StoreysFallen,
            pub part:          crate::armor::BodyPart,
        pub target:        TargetGanger<'a>,
        pub target_entity: Entity,
}

pub(crate) struct FallWoundEnv<'a> {
        pub tuning:       &'a CombatTuning,
        pub tables:       &'a InjuryTables,
        pub registry:     &'a InjuryRegistry,
        pub severity_rng: &'a mut SeverityRng,
        pub injury_rng:   &'a mut InjuryRng,
}

/// so the signature passes clippy's argument-count gate with NO `#[expect]` suppression
pub(crate) fn resolve_fall_hit(
    impact: FallImpact<'_>,
    env: FallWoundEnv<'_>,
) -> Option<RolledInjury> {
    let FallImpact {
        per_storey,
        storeys,
        part,
        target,
        target_entity,
    } = impact;
    let FallWoundEnv {
        tuning,
        tables,
        registry,
        severity_rng,
        injury_rng,
    } = env;

    let damage = fall_damage_magnitude(per_storey, storeys);

    let piece = match target.piece.as_ref() {
        Some(p) if *p.protects() => ArmorPiece::new(
            p.floor,
            p.protection,
            p.integrity_value(),
            p.hardness,
            p.armor_type,
        ),
        _ => BARE_FLESH,
    };

    synthesize_wound(WoundCoreInputs {
        blow: WoundBlow {
            part,
            damage,
            punch: WeaponPunch::new(0),
            shred: WeaponShred::new(0),
            piece,
            matchup: Matchup::Neutral,
            fatal_bias: FatalBias::new(0.0),
            shooter_luck: Luck::new(0.0),
            context: DamageContext::Fall,
            damage_mult: None,
        },
        target,
        target_entity,
        tuning,
        severity_rng,
        tables,
        registry,
        injury_rng,
    })
    .and_then(|synthesis| synthesis.injury)
}
