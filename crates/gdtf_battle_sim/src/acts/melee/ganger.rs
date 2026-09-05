use bevy::prelude::Entity;

use super::{
    MeleeCombatants, MeleeOutcomes, MeleeWorld,
    cost::{MeleeReach, can_melee},
    emit::emit_connect_signals,
    snapshot::{AttackerSnapshot, DefenderResilience, MeleeStreams},
};
use crate::{
    armor::WornArmor,
    ganger::{effective_luck, effective_toughness},
    injuries::{InjuryRegistry, InjuryTables},
    los::{Observer, PeekOffset, Target, has_los},
    march::MarchGrids,
    melee::{Combatants, MeleeStrike, MeleeStrikeEnv, MeleeWeaponHit, resolve_melee_strike},
    metric::CellLevel,
    resolve_and_apply::{StruckPiece, TargetGanger},
    tu::spend_tu,
};

pub(super) fn resolve_ganger_melee(
    attacker: &AttackerSnapshot<'_>,
    target_entity: Entity,
    combatants: &mut MeleeCombatants,
    armor: &mut WornArmor,
    world: &mut MeleeWorld,
    streams: MeleeStreams<'_>,
    outcomes: &mut MeleeOutcomes,
) {
    let Ok((&tgt_pos, &tgt_stance, _, &tgt_fight, &tgt_faction, _)) =
        combatants.geom.get(target_entity)
    else {
        return;
    };

    let Ok((_, _, &tgt_life, _, &tgt_toughness, &tgt_luck, tgt_injuries)) =
        combatants.targets.get(target_entity)
    else {
        return;
    };

    let reach = MeleeReach::ganger(tgt_pos, tgt_faction, tgt_life);
    if !*can_melee(attacker.reach(), reach) {
        return;
    }
    let resilience = match tgt_injuries {
        Some(ledger) => DefenderResilience {
            toughness: effective_toughness(tgt_toughness, ledger),
            luck:      effective_luck(tgt_luck, ledger),
        },
        None => DefenderResilience {
            toughness: tgt_toughness,
            luck:      tgt_luck,
        },
    };

    let observer = Observer {
        position:         &attacker.position,
        stance:           &attacker.stance,
        facing:           &attacker.facing,
        stair_eye_offset: world.occupancy.stair_eye_offset_at(&attacker.position),
        peek_offset:      PeekOffset::default(),
    };
    let los_target = Target {
        position: &tgt_pos,
        stance:   &tgt_stance,
    };
    let sighted = {
        let is_floored = |entity: Entity| {
            combatants
                .targets
                .get(entity)
                .is_ok_and(|(_, _, life, ..)| !*life.is_active())
        };
        has_los(
            &observer,
            &los_target,
            MarchGrids {
                occupancy: &world.occupancy,
                surface:   &world.surface,
                cover:     &world.cover,
            },
            &world.tuning,
            is_floored,
        )
    };
    if !*sighted {
        return;
    }

    let Ok(mut attacker_tu) = combatants.tu.get_mut(attacker.entity) else {
        return;
    };
    if spend_tu(&mut attacker_tu, attacker.tu_cost).is_err() {
        return;
    }

    let empty_tables = InjuryTables::default();
    let empty_registry = InjuryRegistry::default();
    let tables: &InjuryTables = world.tables.as_deref().unwrap_or(&empty_tables);
    let registry: &InjuryRegistry = world.registry.as_deref().unwrap_or(&empty_registry);

    let strike = strike_with_target(
        combatants,
        armor,
        target_entity,
        Combatants {
            attacker_fight: attacker.fight,
            defender_fight: tgt_fight,
            attacker_luck:  attacker.luck,
        },
        resilience,
        attacker.weapon,
        MeleeStrikeEnv {
            tuning: &world.tuning,
            tables,
            registry,
            fight_rng: streams.fight,
            shot_rng: streams.shot,
            severity_rng: streams.severity,
            injury_rng: streams.injury,
        },
    );

    if *strike.connect {
        let at: CellLevel = *tgt_pos;
        emit_connect_signals(
            attacker,
            target_entity,
            at,
            strike,
            &combatants.targets,
            outcomes,
        );
    }
}

fn strike_with_target(
    combatants: &mut MeleeCombatants,
    armor: &mut WornArmor,
    target_entity: Entity,
    contest: Combatants,
    resilience: DefenderResilience,
    weapon: MeleeWeaponHit<'_>,
    env: MeleeStrikeEnv<'_>,
) -> MeleeStrike {
    let piece_view = armor
        .wears
        .get(target_entity)
        .ok()
        .and_then(|worn| worn.pieces().next())
        .and_then(|piece_entity| {
            armor
                .pieces
                .get_mut(piece_entity)
                .ok()
                .map(|piece| StruckPiece {
                    floor:      *piece.floor,
                    protection: *piece.protection,
                    hardness:   *piece.hardness,
                    armor_type: *piece.armor_type,
                    integrity:  piece.integrity.into_inner(),
                })
        });

    let Ok((mut hp, mut wounds, mut life, mut inflicted, ..)) =
        combatants.targets.get_mut(target_entity)
    else {
        return MeleeStrike {
            connect:   crate::melee::Connected::new(false),
            severity:  crate::severity::Severity::None,
            hp_damage: crate::resolve_hit::HpDamage::new(0),
            wear:      crate::armor_wear::ArmorWearOutcome::Unaffected,
            injury:    None,
        };
    };
    resolve_melee_strike(
        contest,
        weapon,
        TargetGanger {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            piece:     piece_view,
            inflicted: &mut inflicted,
            toughness: resilience.toughness,
            luck:      resilience.luck,
        },
        target_entity,
        env,
    )
}
