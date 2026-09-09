//! Enemy turn brain: a downed act if one is adjacent, else fire, else reload, else melee, else
//! advance, else a door, else crouch, else end turn.

use bevy::prelude::{Entity, Query, Res};

use super::{
    advance::plan_reposition,
    decide::{AiTarget, pick_nearest},
    door::{door_rows, plan_door, write_door_act},
    downed::{plan_downed_act, write_downed_act},
    engage::{WeaponLookup, engageable_targets},
    params::{AiActRequests, AiPlanningGrids, AiTurnSides},
    posture::{plan_aim, plan_crouch},
    snapshot::{EnemyTurnGangers, GangerRow, cell_order, ganger_rows},
};
use crate::{
    acts::{
        EndTurnRequested, FireRequested, MeleeAttacker, MeleeReach, MeleeRequested, MoveRequested,
        ReloadRequested, can_melee, can_reload, melee_tu_cost,
        movement::{BreakAwayMover, suppressed_move_legal},
    },
    ganger::{Faction, LifeState},
    los::{Observer, PeekOffset, Target, has_los},
    magazine::{Magazine, mode_tu_cost},
    terrain::{entity::TerrainCell, openable::OpenState},
    tu::can_spend_tu,
    visibility::OmniscientFog,
    weapon::Handedness,
};

// A target this enemy can engage right now, as a fire request.
fn plan_shot(
    enemy: &GangerRow,
    targets: &[GangerRow],
    weapons: &WeaponLookup,
    grids: &AiPlanningGrids,
    is_floored: &impl Fn(Entity) -> bool,
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
        is_floored,
    );
    let target = pick_nearest(enemy.position.cell(), enemy.position.level(), &engageable)?;
    Some(FireRequested::new(
        enemy.entity,
        mode,
        target.cell,
        target.level,
    ))
}

// Empty mag, same `can_reload` gate the player dispatch uses, as a reload request.
fn plan_reload(enemy: &GangerRow, weapons: &WeaponLookup) -> Option<ReloadRequested> {
    let (magazine, ..) = weapons.firing(enemy.entity)?;
    if !*magazine.is_empty() {
        return None;
    }
    (*can_reload(enemy.life, &enemy.tu, magazine)).then(|| ReloadRequested::new(enemy.entity))
}

// Adjacent living foe in LOS the sim would accept, as a melee request.
fn plan_melee(
    enemy: &GangerRow,
    targets: &[GangerRow],
    weapons: &WeaponLookup,
    grids: &AiPlanningGrids,
    is_floored: &impl Fn(Entity) -> bool,
) -> Option<MeleeRequested> {
    let fight_mode = weapons.melee_fight_mode(enemy.entity)?;
    let cost = melee_tu_cost(fight_mode);
    if !*can_spend_tu(&enemy.tu, cost) {
        return None;
    }
    let attacker = MeleeAttacker::new(enemy.position, enemy.faction);
    let observer = Observer {
        position:         &enemy.position,
        stance:           &enemy.stance,
        facing:           &enemy.facing,
        stair_eye_offset: grids.march().occupancy.stair_eye_offset_at(&enemy.position),
        peek_offset:      PeekOffset::default(),
    };
    let mut swingable: Vec<AiTarget> = Vec::new();
    for target in targets {
        let reach = MeleeReach::ganger(target.position, target.faction, target.life);
        if !*can_melee(attacker, reach) {
            continue;
        }
        let los_target = Target {
            position: &target.position,
            stance:   &target.stance,
        };
        if !*has_los(
            &observer,
            &los_target,
            grids.march(),
            grids.tuning(),
            is_floored,
        ) {
            continue;
        }
        swingable.push(AiTarget::new(
            target.entity,
            target.position.cell(),
            target.position.level(),
        ));
    }
    let picked = pick_nearest(enemy.position.cell(), enemy.position.level(), &swingable)?;
    Some(MeleeRequested::new(enemy.entity, picked.entity))
}

// One step closer to the nearest target, as a move request the sim will not turn down.
fn plan_step(
    enemy: &GangerRow,
    rows: &[GangerRow],
    all_targets: &[AiTarget],
    omniscient: Option<&OmniscientFog>,
    grids: &AiPlanningGrids,
    is_floored: &impl Fn(Entity) -> bool,
) -> Option<MoveRequested> {
    let goal = pick_nearest(enemy.position.cell(), enemy.position.level(), all_targets)?;
    let departure = grids.departure(*enemy.position, enemy.seat);
    let dest = plan_reposition(
        enemy,
        &goal,
        rows,
        omniscient?,
        grids.routes(),
        &departure,
        grids.surcharge(enemy.seat),
    )?;
    if let Some(suppressor) = enemy.pinned_by {
        let mover =
            BreakAwayMover::new(enemy.entity, &enemy.position, &enemy.stance, &enemy.facing);
        if !*suppressed_move_legal(
            &mover,
            &dest,
            &suppressor,
            grids.cover(),
            &grids.sight(is_floored),
        ) {
            return None;
        }
    }
    Some(MoveRequested::new(enemy.entity, dest))
}

fn split_sides(rows: &[GangerRow], active: Faction) -> (Vec<GangerRow>, Vec<GangerRow>) {
    let mut enemies: Vec<GangerRow> = rows
        .iter()
        .copied()
        .filter(|row| row.faction == active)
        .collect();
    enemies.sort_by_key(|row| cell_order(&row.position));
    let targets = rows
        .iter()
        .copied()
        .filter(|row| row.faction != active)
        .collect();
    (enemies, targets)
}

/// One enemy acts: execute or stabilize an adjacent downed ganger, aim, shoot, reload, melee,
/// move closer, open a door, crouch, or end the turn.
pub fn enemy_ai_turn(
    sides: AiTurnSides,
    omniscient: Option<Res<OmniscientFog>>,
    grids: AiPlanningGrids,
    gangers: EnemyTurnGangers,
    weapon_lookup: WeaponLookup,
    doors: Query<(Entity, &OpenState, &TerrainCell)>,
    mut orders: AiActRequests,
) {
    let Some(active) = sides.enemy_faction() else {
        return;
    };
    let rows = ganger_rows(&gangers);
    let (enemies, targets) = split_sides(&rows, active);
    // Dead only, not `is_active`: a downed enemy is still worth walking up to.
    let all_targets: Vec<AiTarget> = targets
        .iter()
        .filter(|row| row.life != LifeState::Dead)
        .map(|row| AiTarget::new(row.entity, row.position.cell(), row.position.level()))
        .collect();
    let scanned = door_rows(
        doors
            .iter()
            .map(|(entity, state, cell)| (entity, *state, **cell)),
    );
    let is_floored_fn = |entity: Entity| {
        rows.iter()
            .find(|row| row.entity == entity)
            .is_some_and(|row| !*row.life.is_active())
    };
    let is_floored = &is_floored_fn;
    let fog = omniscient.as_deref();
    let mut acted = false;
    for enemy in &enemies {
        if !*enemy.life.is_active() || *enemy.walking {
            continue;
        }
        if let Some(downed) = plan_downed_act(enemy, &rows, &grids) {
            write_downed_act(&mut orders, downed);
            acted = true;
            break;
        }
        if let Some(aim) = plan_aim(enemy, &targets, &weapon_lookup, &grids, is_floored) {
            orders.aim.write(aim);
            acted = true;
            break;
        }
        if let Some(shot) = plan_shot(enemy, &targets, &weapon_lookup, &grids, is_floored) {
            orders.fire.write(shot);
            acted = true;
            break;
        }
        if let Some(reload) = plan_reload(enemy, &weapon_lookup) {
            orders.reload.write(reload);
            acted = true;
            break;
        }
        if let Some(swing) = plan_melee(enemy, &targets, &weapon_lookup, &grids, is_floored) {
            orders.melee.write(swing);
            acted = true;
            break;
        }
        if let Some(step) = plan_step(enemy, &rows, &all_targets, fog, &grids, is_floored) {
            orders.step.write(step);
            acted = true;
            break;
        }
        if let Some(door) = plan_door(enemy, &scanned, &rows, fog, &grids, is_floored) {
            write_door_act(&mut orders, door);
            acted = true;
            break;
        }
        if let Some(crouch) = plan_crouch(enemy, &grids) {
            orders.stance.write(crouch);
            acted = true;
            break;
        }
    }
    if !acted && !enemies.iter().any(|enemy| *enemy.walking) {
        orders.end_turn.write(EndTurnRequested);
    }
}
