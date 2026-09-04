//! A shot that has to turn first: what `battle.cost` quotes against what `act.fire` charges.

use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_protocol::{command::RunOptions, message::McpRequest};
use gdtf_battle_sim::{
    combatants::firing_arc::target_in_arc,
    ganger::{Direction, Facing, Tu},
    tuning::CombatTuning,
    weapon::FireModeSpec,
};
use gdtf_game::qa_wire::{
    cell::CellLevelNet,
    cost::{CostActNet, CostRefusalNet},
    misc::ModeKindNet,
    vitals::TuNet,
};

use super::support::{a_gun_with_modes, cost_body, cost_call, fire_cost, settle};
use crate::{
    act_support::{accepted, caught_up, next, selected},
    battle_reads::{an_enemy_ganger_at, cell_argument, cell_of, ganger_argument, token_of},
    command_exchange::{
        ACT_FIRE, ACT_SELECT, BATTLE_COST, BATTLE_SET_FIRE_MODE, WAIT, exchange_in_battle,
        ran_body, run,
    },
    socket_support::TestResult,
};

/// Why the fixture cannot host this case.
const NO_SHOT: &str =
    "the fixture must field a player ganger holding a ranged weapon and a living enemy to shoot at";

/// The shot a case sets up: who fires, at what, in which mode, and the pool it starts with.
struct TurningShot {
    shooter:   Entity,
    target:    CellLevelNet,
    spec:      FireModeSpec,
    tu_before: Tu,
}

#[test]
fn a_fire_that_has_to_turn_first_is_quoted_the_whole_charge() -> TestResult {
    let mut planned: Option<TurningShot> = None;
    let (app, replies) = exchange_in_battle(|app| {
        settle(app);
        let Some(shot) = a_turning_shot(app) else {
            return Vec::new();
        };
        let asked = asked_for(&shot);
        planned = Some(shot);
        asked
    })?;
    let Some(shot) = planned else {
        return Err(NO_SHOT.into());
    };

    let mut replies = replies.into_iter();
    let _waited = ran_body(WAIT, next(WAIT, &mut replies)?)?;
    let shooter = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let _mode = ran_body(
        BATTLE_SET_FIRE_MODE,
        next(BATTLE_SET_FIRE_MODE, &mut replies)?,
    )?;
    let body = cost_body(next(BATTLE_COST, &mut replies)?)?;
    let _fired = accepted(ACT_FIRE, next(ACT_FIRE, &mut replies)?)?;

    assert_eq!(
        shooter,
        Some(token_of(shot.shooter)),
        "the shot comes from the ganger the quote was asked about, so selection must have taken",
    );
    assert_eq!(
        body.refusal, None,
        "a shot the shooter can pay for and turn into carries no refusal: {body:?}",
    );
    assert!(
        *body.legal,
        "the shot the case takes must be legal: {body:?}"
    );

    let Some(after) = tu_of(&app, shot.shooter) else {
        return Err("the shooter the case fired with must still be there".into());
    };
    let Some(charged) = charged(shot.tu_before, after) else {
        return Err("a shot only ever takes TU out of the pool, it never puts any back".into());
    };
    assert!(
        *charged > 0,
        "the case asks its question only if the shot was actually taken and charged for",
    );
    assert_eq!(
        body.cost,
        Some(charged),
        "the quote must be the whole charge, turn and shot together, not the shot alone",
    );
    Ok(())
}

#[test]
fn a_fire_that_cannot_pay_for_its_turn_is_refused_and_charges_nothing() -> TestResult {
    let mut planned: Option<TurningShot> = None;
    let (app, replies) = exchange_in_battle(|app| {
        settle(app);
        let Some(shot) = a_shot_that_cannot_pay_for_its_turn(app) else {
            return Vec::new();
        };
        let asked = asked_for(&shot);
        planned = Some(shot);
        asked
    })?;
    let Some(shot) = planned else {
        return Err(NO_SHOT.into());
    };

    let mut replies = replies.into_iter();
    let _waited = ran_body(WAIT, next(WAIT, &mut replies)?)?;
    let _shooter = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let _mode = ran_body(
        BATTLE_SET_FIRE_MODE,
        next(BATTLE_SET_FIRE_MODE, &mut replies)?,
    )?;
    let body = cost_body(next(BATTLE_COST, &mut replies)?)?;
    let _fired = accepted(ACT_FIRE, next(ACT_FIRE, &mut replies)?)?;

    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::CannotAfford),
        "a pool that covers the shot but not the turn cannot pay for the act: {body:?}",
    );
    assert!(!*body.legal, "a refused act is not legal: {body:?}");
    assert!(
        body.cost
            .is_some_and(|cost| *cost > *TuNet::new(*shot.tu_before)),
        "the price still stands, and it is more than the pool holds: {body:?}",
    );

    let Some(after) = tu_of(&app, shot.shooter) else {
        return Err("the shooter the case fired with must still be there".into());
    };
    assert_eq!(
        charged(shot.tu_before, after),
        Some(TuNet::new(0)),
        "the sim turns the shooter down, so the act takes nothing out of the pool",
    );
    Ok(())
}

/// Point the shooter away from the enemy, so the shot it is then quoted has to turn first.
fn a_turning_shot(app: &mut App) -> Option<TurningShot> {
    let (shooter, modes) = a_gun_with_modes(app)?;
    let (_, target) = an_enemy_ganger_at(app)?;
    let spec = *modes.first()?;
    let away = facing_out_of_arc(app, shooter, target)?;
    app.world_mut()
        .get_entity_mut(shooter)
        .ok()?
        .insert(Facing::new(away));
    settle(app);
    Some(TurningShot {
        shooter,
        target,
        spec,
        tu_before: tu_of(app, shooter)?,
    })
}

/// The same shot, with the pool cut to the bare shot cost so it cannot pay for the turn as well.
fn a_shot_that_cannot_pay_for_its_turn(app: &mut App) -> Option<TurningShot> {
    let shot = a_turning_shot(app)?;
    let bare = fire_cost(app, shot.shooter, &shot.spec)?;
    app.world_mut()
        .get_entity_mut(shot.shooter)
        .ok()?
        .insert(bare);
    settle(app);
    Some(TurningShot {
        tu_before: bare,
        ..shot
    })
}

/// The way back from the target to the shooter, which the sim's own arc test puts out of arc.
fn facing_out_of_arc(app: &App, shooter: Entity, target: CellLevelNet) -> Option<Direction> {
    let actor_cell = cell_of(app, shooter)?.to_sim().cell();
    let target_cell = target.to_sim().cell();
    let tuning = app.world().get_resource::<CombatTuning>()?;
    let away = Direction::from_cells(target_cell, actor_cell)?;
    assert!(
        !*target_in_arc(away, actor_cell, target_cell, &tuning.firing_arc),
        "this case only asks its question about a shot that starts out of arc",
    );
    Some(away)
}

/// Select the shooter, pin the mode the shot will use, quote it, then take it.
fn asked_for(shot: &TurningShot) -> Vec<McpRequest> {
    let mode = ModeKindNet::from_sim(shot.spec.kind);
    let (Some(argument), Some(quote)) = (
        mode_argument(mode),
        cost_call(
            token_of(shot.shooter),
            &CostActNet::Fire {
                target: shot.target,
                mode,
            },
        ),
    ) else {
        return Vec::new();
    };
    vec![
        caught_up(),
        run(
            ACT_SELECT,
            &ganger_argument(shot.shooter),
            RunOptions::default(),
        ),
        run(BATTLE_SET_FIRE_MODE, &argument, RunOptions::default()),
        quote,
        run(ACT_FIRE, &cell_argument(shot.target), RunOptions::default()),
    ]
}

/// Render a fire mode kind as the compact RON body `battle.set_fire_mode` takes.
fn mode_argument(mode: ModeKindNet) -> Option<String> {
    ron::ser::to_string(&mode)
        .ok()
        .map(|text| format!("(mode:{text})"))
}

/// What one ganger has left to spend, read straight off the sim's own component.
fn tu_of(app: &App, ganger: Entity) -> Option<Tu> {
    app.world().get_entity(ganger).ok()?.get::<Tu>().copied()
}

/// What the act took out of the pool, when the pool only ever went down.
fn charged(before: Tu, after: Tu) -> Option<TuNet> {
    (*before).checked_sub(*after).map(TuNet::new)
}
