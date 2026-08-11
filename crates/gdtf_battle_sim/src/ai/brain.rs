//! Enemy turn brain: fire if possible, else advance, else end turn.

use bevy::prelude::{Entity, Res};

use super::{
    advance::plan_reposition,
    decide::{AiTarget, pick_nearest},
    engage::{WeaponLookup, engageable_targets},
    params::{AiActRequests, AiPlanningGrids},
    snapshot::{EnemyTurnGangers, GangerRow, cell_order, ganger_rows},
};
use crate::{
    acts::{
        EndTurnRequested, FireRequested, MoveRequested,
        movement::{BreakAwayMover, suppressed_move_legal},
    },
    battle::PlayerFaction,
    ganger::LifeState,
    magazine::{Magazine, mode_tu_cost},
    turn::ActiveFaction,
    visibility::OmniscientFog,
    weapon::Handedness,
};

// A target this enemy can engage right now, as a fire request.
fn plan_shot(
    enemy: &GangerRow,
    targets: &[GangerRow],
    weapons: &WeaponLookup,
    grids: &AiPlanningGrids,
    is_dead: &impl Fn(Entity) -> bool,
) -> Option<FireRequested> {
    let (magazine, fire_mode, handedness) = weapons.firing(enemy.entity)?;
    let mode = fire_mode.single();
    let magazine: Magazine = *magazine;
    let handedness: Handedness = *handedness;
    let fire_cost = mode_tu_cost(&mode, &enemy.tu_max, &enemy.aiming, grids.tuning());
    let engageable = engageable_targets(
        enemy,
        targets,
        (magazine, mode, handedness),
        fire_cost,
        grids.march(),
        grids.tuning(),
        is_dead,
    );
    let target = pick_nearest(enemy.position.cell(), enemy.position.level(), &engageable)?;
    Some(FireRequested::new(
        enemy.entity,
        mode,
        target.cell,
        target.level,
    ))
}

// One step closer to the nearest target, as a move request the sim will not turn down.
fn plan_step(
    enemy: &GangerRow,
    rows: &[GangerRow],
    all_targets: &[AiTarget],
    omniscient: Option<&OmniscientFog>,
    grids: &AiPlanningGrids,
    is_dead: &impl Fn(Entity) -> bool,
) -> Option<MoveRequested> {
    let goal = pick_nearest(enemy.position.cell(), enemy.position.level(), all_targets)?;
    let dest = plan_reposition(enemy, &goal, rows, omniscient?, grids.routes())?;
    if let Some(suppressor) = enemy.pinned_by {
        let mover =
            BreakAwayMover::new(enemy.entity, &enemy.position, &enemy.stance, &enemy.facing);
        if !*suppressed_move_legal(
            &mover,
            &dest,
            &suppressor,
            grids.cover(),
            &grids.sight(is_dead),
        ) {
            return None;
        }
    }
    Some(MoveRequested::new(enemy.entity, dest))
}

/// One enemy acts: shoot an engageable target, move closer, or end the turn.
pub fn enemy_ai_turn(
    active: Res<ActiveFaction>,
    player: Option<Res<PlayerFaction>>,
    omniscient: Option<Res<OmniscientFog>>,
    grids: AiPlanningGrids,
    gangers: EnemyTurnGangers,
    weapon_lookup: WeaponLookup,
    mut orders: AiActRequests,
) {
    let Some(player_faction) = player.as_deref().map(|player| **player) else {
        return;
    };
    let active_faction = **active;
    if active_faction == player_faction {
        return;
    }

    let rows = ganger_rows(&gangers);

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

        if let Some(shot) = plan_shot(enemy, &targets, &weapon_lookup, &grids, is_dead) {
            orders.fire.write(shot);
            acted = true;
            break;
        }

        if let Some(step) = plan_step(
            enemy,
            &rows,
            &all_targets,
            omniscient.as_deref(),
            &grids,
            is_dead,
        ) {
            orders.step.write(step);
            acted = true;
            break;
        }
    }

    let any_walking = enemies.iter().any(|enemy| *enemy.walking);
    if !acted && !any_walking {
        orders.end_turn.write(EndTurnRequested);
    }
}
