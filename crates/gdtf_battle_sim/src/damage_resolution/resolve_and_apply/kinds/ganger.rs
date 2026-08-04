//! Apply a shot that struck a living combatant.

use bevy::prelude::Entity;

use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType, BodyPart,
    },
    armor_wear::ArmorWearOutcome,
    ganger::LifeState,
    injuries::{DamageContext, RolledInjury},
    matchup::{Matchup, matchup},
    resolve_and_apply::{
        report::{HitVerdict, ShotSource, StruckPiece, TargetGanger, WoundRoll},
        wound_core::{WoundBlow, WoundCoreInputs, synthesize_wound},
    },
    resolve_coarse::ShotOutcome,
    resolve_hit::HitResult,
    severity::Severity,
    weapon::{Dot, WeaponStats},
};

const BARE_FLESH: ArmorPiece = ArmorPiece::new(
    ArmorFloor::new(0),
    ArmorProtection::new(0),
    ArmorIntegrity::new(0),
    ArmorHardness::new(0),
    ArmorType::DEFAULT,
);

/// Damage that was applied to the combatant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedDamage {
    /// Matchup used for the hit.
    pub matchup:    Matchup,
    /// Hit resolution result.
    pub hit:        HitResult,
    /// Wound severity.
    pub severity:   Severity,
    /// Life state after the hit.
    pub life_after: LifeState,
    /// Armor wear outcome.
    pub wear:       ArmorWearOutcome,
}

/// Full verdict for a ganger hit.
#[derive(Debug, Clone, PartialEq)]
pub struct GangerVerdict {
    /// Target entity.
    pub target:      Entity,
    /// Body part that was hit.
    pub part:        BodyPart,
    /// Applied damage details.
    pub applied:     AppliedDamage,
    /// Injury that was rolled, if any.
    pub injury:      Option<RolledInjury>,
    /// DOT that was applied, if any.
    pub dot_applied: Option<Dot>,
}

fn struck_piece(piece: Option<&StruckPiece<'_>>, weapon: WeaponStats<'_>) -> (ArmorPiece, Matchup) {
    match piece {
        Some(p) if *p.protects() => {
            let assembled = ArmorPiece::new(
                p.floor,
                p.protection,
                p.integrity_value(),
                p.hardness,
                p.armor_type,
            );
            let resolved = matchup(*weapon.damage_type, p.armor_type);
            (assembled, resolved)
        }
        _ => (BARE_FLESH, Matchup::Neutral),
    }
}

/// Resolve and apply a ganger hit, or return `NoEffect` when the target is missing/dead.
pub(in crate::damage_resolution::resolve_and_apply) fn fold(
    outcome: &ShotOutcome,
    source: ShotSource<'_>,
    target: Option<TargetGanger<'_>>,
    target_entity: Entity,
    roll: &mut WoundRoll<'_>,
) -> HitVerdict {
    let weapon = source.weapon;
    let Some(target) = target else {
        return HitVerdict::NoEffect;
    };

    let Some(part) = outcome.body_part else {
        return HitVerdict::NoEffect;
    };

    let (piece, resolved_matchup) = struck_piece(target.piece.as_ref(), weapon);

    let Some(synthesis) = synthesize_wound(WoundCoreInputs {
        blow: WoundBlow {
            part,
            damage: *weapon.damage,
            punch: *weapon.punch,
            shred: *weapon.shred,
            piece,
            matchup: resolved_matchup,
            fatal_bias: *weapon.fatal_bias,
            shooter_luck: source.luck,
            context: DamageContext::Ranged,
            damage_mult: None,
        },
        target,
        target_entity,
        tuning: roll.tuning,
        severity_rng: &mut *roll.severity_rng,
        tables: roll.tables,
        registry: roll.registry,
        injury_rng: &mut *roll.injury_rng,
    }) else {
        return HitVerdict::NoEffect;
    };

    let dot_applied = weapon
        .dot
        .filter(|_| *synthesis.hit.penetrating > 0)
        .map(|profile| Dot::from_profile(*profile));

    HitVerdict::Ganger(Box::new(GangerVerdict {
        target: target_entity,
        part,
        applied: AppliedDamage {
            matchup:    synthesis.matchup,
            hit:        synthesis.hit,
            severity:   synthesis.severity,
            life_after: synthesis.life_after,
            wear:       synthesis.wear,
        },
        injury: synthesis.injury,
        dot_applied,
    }))
}
