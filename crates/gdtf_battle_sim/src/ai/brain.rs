//! Enemy turn brain: fire if possible, else advance, else end turn.

use bevy::prelude::{Entity, MessageWriter, Res};

use super::{
    advance::plan_reposition,
    decide::{AiTarget, pick_nearest},
    engage::{WeaponLookup, engageable_targets},
    snapshot::{EnemyTurnGangers, GangerRow, MidWalk, cell_order},
};
use crate::{
    acts::{EndTurnRequested, FireRequested, MoveRequested},
    battle::PlayerFaction,
    cover::CoverLedger,
    ganger::LifeState,
    injuries::{HandsAvailable, InflictedInjuries, MovementCostFactor},
    magazine::{Magazine, mode_tu_cost},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
    terrain::floor::FloorCostGrid,
    tuning::CombatTuning,
    turn::ActiveFaction,
    vertical::VerticalLinkGraph,
    visibility::OmniscientFog,
    weapon::Handedness,
};

/// One enemy acts: shoot an engageable target, move closer, or end the turn.
#[expect(
    clippy::too_many_arguments,
    reason = "the enemy brain reads the active/player factions, the combat tuning, the five \
              read grids (occupancy / surface / cover / links / floor costs) + the AI move \
              fog, the ganger + wielded-weapon + \
              weapon-entity queries, and the three act MessageWriters; each is a distinct \
              Bevy SystemParam, mirroring dispatch_fire's own argument-count carve-out, and \
              bundling them would only hide the real reads"
)]
#[expect(
    clippy::too_many_lines,
    reason = "the brain is ONE cohesive per-frame engage-or-advance-or-hold pass over the \
              enemy gangers (snapshot → sorted decision loop → emission-decoupled turn-end); \
              splitting it would thread the grids/tuning/snapshot + the two borrowed closures \
              (is_dead / relation_of) through helpers, obscuring the access set more than the \
              length costs (the setup_battle too_many_lines precedent)"
)]
pub fn enemy_ai_turn(
    active: Res<ActiveFaction>,
    player: Option<Res<PlayerFaction>>,
    tuning: Res<CombatTuning>,
    occupancy: Res<OccupancyGrid>,
    surface: Res<SurfaceGrid>,
    cover: Res<CoverLedger>,
    links: Res<VerticalLinkGraph>,
    floor_costs: Res<FloorCostGrid>,
    omniscient: Option<Res<OmniscientFog>>,
    gangers: EnemyTurnGangers,
    weapon_lookup: WeaponLookup,
    mut fire_writer: MessageWriter<FireRequested>,
    mut move_writer: MessageWriter<MoveRequested>,
    mut end_turn_writer: MessageWriter<EndTurnRequested>,
) {
    let Some(player_faction) = player.as_deref().map(|player| **player) else {
        return;
    };
    let active_faction = **active;
    if active_faction == player_faction {
        return;
    }

    let rows: Vec<GangerRow> = gangers
        .iter()
        .map(
            |(
                entity,
                position,
                stance,
                facing,
                aiming,
                life,
                tu,
                tu_max,
                faction,
                walking,
                injuries,
            )| {
                GangerRow {
                    entity,
                    position: *position,
                    stance: *stance,
                    facing: *facing,
                    aiming: *aiming,
                    life: *life,
                    tu: *tu,
                    tu_max: *tu_max,
                    faction: *faction,
                    walking: MidWalk::new(walking),
                    hands: injuries
                        .map_or_else(HandsAvailable::default, InflictedInjuries::hands_available),
                    factor: injuries.map_or(
                        MovementCostFactor::IDENTITY,
                        InflictedInjuries::movement_cost_factor,
                    ),
                }
            },
        )
        .collect();

    let mut enemies: Vec<GangerRow> = rows
        .iter()
        .copied()
        .filter(|row| row.faction == active_faction)
        .collect();
    enemies.sort_by_key(|row| cell_order(&row.position));

    let targets: Vec<GangerRow> = rows
        .iter()
        .copied()
        .filter(|row| row.faction != active_faction)
        .collect();
    let all_targets: Vec<AiTarget> = targets
        .iter()
        .map(|row| AiTarget::new(row.entity, row.position.cell(), row.position.level()))
        .collect();

    let is_dead_fn = |entity: Entity| {
        rows.iter()
            .find(|row| row.entity == entity)
            .is_some_and(|row| row.life == LifeState::Dead)
    };
    let is_dead = &is_dead_fn;
    let mut acted = false;
    for enemy in &enemies {
        if !*enemy.life.is_active() || *enemy.walking {
            continue;
        }

        let enemy_cell = enemy.position.cell();
        let enemy_level = enemy.position.level();

        let weapon_data = weapon_lookup.firing(enemy.entity);
        if let Some((magazine, fire_mode, handedness)) = weapon_data {
            let mode = fire_mode.single();
            let magazine: Magazine = *magazine;
            let handedness: Handedness = *handedness;
            let fire_cost = mode_tu_cost(&mode, &enemy.tu_max, &enemy.aiming, &tuning);
            let engageable = engageable_targets(
                enemy,
                &targets,
                (magazine, mode, handedness),
                fire_cost,
                &occupancy,
                &surface,
                &cover,
                &tuning,
                is_dead,
            );
            if let Some(target) = pick_nearest(enemy_cell, enemy_level, &engageable) {
                fire_writer.write(FireRequested::new(
                    enemy.entity,
                    mode,
                    target.cell,
                    target.level,
                ));
                acted = true;
                break;
            }
        }

        if let (Some(goal), Some(omniscient)) = (
            pick_nearest(enemy_cell, enemy_level, &all_targets),
            omniscient.as_deref(),
        ) && let Some(dest) = plan_reposition(
            enemy,
            &goal,
            &rows,
            omniscient,
            &occupancy,
            &links,
            &tuning,
            &floor_costs,
        ) {
            move_writer.write(MoveRequested::new(enemy.entity, dest));
            acted = true;
            break;
        }
    }

    let any_walking = enemies.iter().any(|enemy| *enemy.walking);
    if !acted && !any_walking {
        end_turn_writer.write(EndTurnRequested);
    }
}
