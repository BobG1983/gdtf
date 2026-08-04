use bevy::prelude::{Entity, MessageWriter, Query, With};

use super::{
    MeleeFacts, MeleeWorld,
    emit::{MeleeConnectSignals, emit_connect_signals},
    queries::{MeleeGeomQuery, MeleeTargetQuery},
    snapshot::{AttackerSnapshot, MeleeStreams},
};
use crate::{
    acts::{
        downed::is_8_adjacent,
        request::{MeleeResolved, ShoveRequested},
    },
    armor::{PieceArmorMut, Wears, WornBy},
    ganger::{LifeState, Luck, Toughness, Tu, effective_luck, effective_toughness},
    injuries::{InjuryRegistry, InjuryTables},
    los::{Observer, PeekOffset, Target, has_los},
    march::MarchGrids,
    melee::{Combatants, MeleeStrike, MeleeStrikeEnv, MeleeWeaponHit, resolve_melee_strike},
    metric::CellLevel,
    resolve_and_apply::{StruckPiece, TargetGanger},
    tu::spend_tu,
};

#[expect(
    clippy::too_many_arguments,
    reason = "the ganger arm threads the attacker snapshot, the target entity, the disjoint \
              geometry / attacker-Tu / target-surfaces queries, the two armor relationship \
              queries, the grouped world reads, the four draw streams, and the \
              MeleeResolved + ShoveRequested writers and the grouped fact writers \
              (MeleeFacts) — the irreducible per-arm access set (the strike_with_target \
              precedent); bundling further would only hide the access set"
)]
pub(super) fn resolve_ganger_melee(
    attacker: &AttackerSnapshot<'_>,
    target_entity: Entity,
    geom: &MeleeGeomQuery,
    tu_q: &mut Query<&mut Tu>,
    targets: &mut MeleeTargetQuery,
    wears: &Query<&Wears>,
    pieces: &mut Query<PieceArmorMut, With<WornBy>>,
    world: &mut MeleeWorld,
    streams: MeleeStreams<'_>,
    resolved: &mut MessageWriter<MeleeResolved>,
    facts: &mut MeleeFacts,
    shoves: &mut MessageWriter<ShoveRequested>,
    deaths: &mut MessageWriter<crate::effects::on_death::OnDeathOccurred>,
) {
    let Ok((&tgt_pos, &tgt_stance, _, &tgt_fight, &tgt_faction, _)) = geom.get(target_entity)
    else {
        return;
    };

    if !*is_8_adjacent(attacker.position, tgt_pos) || attacker.faction == tgt_faction {
        return;
    }

    let Ok((_, _, &tgt_life, _, &tgt_toughness, &tgt_luck, tgt_injuries)) =
        targets.get(target_entity)
    else {
        return;
    };
    if !*tgt_life.is_active() {
        return;
    }
    let (effective_toughness, effective_luck) = match tgt_injuries {
        Some(ledger) => (
            effective_toughness(tgt_toughness, ledger),
            effective_luck(tgt_luck, ledger),
        ),
        None => (tgt_toughness, tgt_luck),
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
    let is_dead = |entity: Entity| {
        targets
            .get(entity)
            .is_ok_and(|(_, _, life, ..)| *life == LifeState::Dead)
    };
    let sighted = has_los(
        &observer,
        &los_target,
        MarchGrids {
            occupancy: &world.occupancy,
            surface:   &world.surface,
            cover:     &world.cover,
        },
        &world.tuning,
        is_dead,
    );
    if !*sighted {
        return;
    }

    let Ok(mut attacker_tu) = tu_q.get_mut(attacker.entity) else {
        return;
    };
    spend_tu(&mut attacker_tu, attacker.tu_cost);

    let empty_tables = InjuryTables::default();
    let empty_registry = InjuryRegistry::default();
    let tables: &InjuryTables = world.tables.as_deref().unwrap_or(&empty_tables);
    let registry: &InjuryRegistry = world.registry.as_deref().unwrap_or(&empty_registry);

    let strike = strike_with_target(
        targets,
        wears,
        pieces,
        target_entity,
        Combatants {
            attacker_fight: attacker.fight,
            defender_fight: tgt_fight,
            attacker_luck:  attacker.luck,
        },
        effective_toughness,
        effective_luck,
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
            targets,
            MeleeConnectSignals {
                resolved,
                facts,
                shoves,
                deaths,
            },
        );
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the fold threads the target query + the two armor relationship queries + the \
              target entity + the combatants + the projected Toughness/Luck + the weapon + \
              the grouped strike environment (tuning + injury content + the four draw \
              streams) — the irreducible input set resolve_melee_strike takes; bundling \
              further would only hide the access set"
)]
fn strike_with_target(
    targets: &mut MeleeTargetQuery,
    wears: &Query<&Wears>,
    pieces: &mut Query<PieceArmorMut, With<WornBy>>,
    target_entity: Entity,
    combatants: Combatants,
    toughness: Toughness,
    luck: Luck,
    weapon: MeleeWeaponHit<'_>,
    env: MeleeStrikeEnv<'_>,
) -> MeleeStrike {
    let piece_view = wears
        .get(target_entity)
        .ok()
        .and_then(|worn| worn.pieces().next())
        .and_then(|piece_entity| {
            pieces.get_mut(piece_entity).ok().map(|piece| StruckPiece {
                floor:      *piece.floor,
                protection: *piece.protection,
                hardness:   *piece.hardness,
                armor_type: *piece.armor_type,
                integrity:  piece.integrity.into_inner(),
            })
        });

    let Ok((mut hp, mut wounds, mut life, mut inflicted, ..)) = targets.get_mut(target_entity)
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
        combatants,
        weapon,
        TargetGanger {
            hp: &mut hp,
            wounds: &mut wounds,
            life: &mut life,
            piece: piece_view,
            inflicted: &mut inflicted,
            toughness,
            luck,
        },
        target_entity,
        env,
    )
}
