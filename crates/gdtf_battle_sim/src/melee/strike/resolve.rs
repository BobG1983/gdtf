//! Resolve opposed fight then wound synthesis for a melee hit.

use crate::{
    armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType},
    hit_location::roll_body_part,
    injuries::DamageContext,
    matchup::{Matchup, matchup},
    melee::{
        Connected, melee_damage_mult, opposed_fight,
        strike::{Combatants, MeleeStrike, MeleeStrikeEnv, MeleeWeaponHit},
    },
    resolve_and_apply::{StruckPiece, TargetGanger, WoundBlow, WoundCoreInputs, synthesize_wound},
};

const BARE_FLESH: ArmorPiece = ArmorPiece::new(
    ArmorFloor::new(0),
    ArmorProtection::new(0),
    ArmorIntegrity::new(0),
    ArmorHardness::new(0),
    ArmorType::DEFAULT,
);

fn melee_struck_piece(
    piece: Option<&StruckPiece<'_>>,
    weapon: MeleeWeaponHit<'_>,
) -> (ArmorPiece, Matchup) {
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

/// Opposed fight → on connect, synthesize wound against the target.
#[must_use]
pub fn resolve_melee_strike(
    combatants: Combatants,
    weapon: MeleeWeaponHit<'_>,
    target: TargetGanger<'_>,
    target_entity: bevy::prelude::Entity,
    env: MeleeStrikeEnv<'_>,
) -> MeleeStrike {
    let MeleeStrikeEnv {
        tuning,
        tables,
        registry,
        fight_rng,
        shot_rng,
        severity_rng,
        injury_rng,
    } = env;

    let part = roll_body_part(&tuning.body_part_weights, shot_rng.rng());

    let outcome = opposed_fight(
        combatants.attacker_fight,
        combatants.defender_fight,
        tuning.melee.variance,
        fight_rng,
    );

    if !*outcome.connect {
        return MeleeStrike::MISS;
    }

    let (piece, resolved_matchup) = melee_struck_piece(target.piece.as_ref(), weapon);
    let mult = melee_damage_mult(outcome.margin, &tuning.melee);

    let Some(synthesis) = synthesize_wound(WoundCoreInputs {
        blow: WoundBlow {
            part,
            damage: *weapon.damage,
            punch: *weapon.punch,
            shred: *weapon.shred,
            piece,
            matchup: resolved_matchup,
            fatal_bias: *weapon.fatal_bias,
            shooter_luck: combatants.attacker_luck,
            context: DamageContext::Melee,
            damage_mult: Some(mult),
        },
        target,
        target_entity,
        tuning,
        severity_rng,
        tables,
        registry,
        injury_rng,
    }) else {
        return MeleeStrike::CORPSE;
    };

    MeleeStrike {
        connect:   Connected::new(true),
        severity:  synthesis.severity,
        hp_damage: synthesis.hit.hp_damage,
        wear:      synthesis.wear,
        injury:    synthesis.injury,
    }
}
