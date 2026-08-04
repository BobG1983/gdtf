//! Shared wound synthesis used by ranged and melee paths.

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

/// Weapon-side inputs for one wound.
pub(crate) struct WoundBlow {
    /// Body part hit.
    pub part:         BodyPart,
    /// Weapon damage.
    pub damage:       WeaponDamage,
    /// Weapon punch.
    pub punch:        WeaponPunch,
    /// Weapon shred.
    pub shred:        WeaponShred,
    /// Armor piece at the location (or bare flesh).
    pub piece:        ArmorPiece,
    /// Damage-type vs armor matchup.
    pub matchup:      Matchup,
    /// Weapon fatal bias.
    pub fatal_bias:   FatalBias,
    /// Shooter luck.
    pub shooter_luck: Luck,
    /// How the damage was caused.
    pub context:      DamageContext,
    /// Optional melee damage multiplier.
    pub damage_mult:  Option<MeleeDamageMult>,
}

/// Full inputs for synthesizing one wound.
pub(crate) struct WoundCoreInputs<'a> {
    /// Weapon-side blow data.
    pub blow:          WoundBlow,
    /// Target combatant view.
    pub target:        TargetGanger<'a>,
    /// Target entity id.
    pub target_entity: Entity,
    /// Combat tuning.
    pub tuning:        &'a CombatTuning,
    /// Severity RNG.
    pub severity_rng:  &'a mut SeverityRng,
    /// Injury tables.
    pub tables:        &'a InjuryTables,
    /// Injury registry.
    pub registry:      &'a InjuryRegistry,
    /// Injury RNG.
    pub injury_rng:    &'a mut InjuryRng,
}

/// Result of synthesizing one wound.
pub(crate) struct WoundSynthesis {
    /// Matchup used.
    pub matchup:    Matchup,
    /// Hit resolution result.
    pub hit:        HitResult,
    /// Rolled severity.
    pub severity:   Severity,
    /// Armor wear outcome.
    pub wear:       ArmorWearOutcome,
    /// Life state after apply.
    pub life_after: LifeState,
    /// Injury rolled, if any.
    pub injury:     Option<RolledInjury>,
}

/// Resolve hit, roll severity, apply damage, and optionally roll an injury.
///
/// Returns `None` when the target is already dead.
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
