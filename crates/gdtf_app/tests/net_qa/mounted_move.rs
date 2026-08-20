//! A mounted ganger's walk over a real socket: what `battle.cost` quotes and what `act.move` spends.

use bevy::app::App;
use gdtf_app::qa_wire::{WaitConditionNet, cell::CellLevelNet, cost::CostActNet, vitals::TuNet};
use gdtf_battle_sim::{
    ganger::Tu,
    prelude::CellLevel,
    tuning::{CombatTuning, ExitEmplacementTu},
};
use gdtf_qa_protocol::{command::RunOptions, message::QaRequest};

use crate::{
    act_support::{
        RosterBody, accepted, assert_caught_up, assert_waited_for, card_of, caught_up, decode,
        walk_complete,
    },
    battle_cost::support::{CostBody, cost_body, cost_call},
    battle_reads::{cell_argument, ganger_argument, token_of, two_steps_from},
    command_exchange::{ACT_MOVE, ACT_SELECT, BATTLE_ROSTER, WAIT, greet, run},
    contextual_acts::emplacement::manned_emplacement_under_the_shooter,
    socket_support::{TestError, TestResult},
};

/// Why the generated map cannot host a walk off the seat.
const NO_ROOM: &str = "the generated map must offer two clear steps in a line off the seat the \
                       fixture put the shooter on";

/// The exit leaf the live battle ships, or a failure when it is zero and proves nothing.
fn authored_exit_tu(app: &App) -> Result<u8, TestError> {
    let Some(tuning) = app.world().get_resource::<CombatTuning>() else {
        return Err(
            "a running battle must hold the tuning a walk off a seat is priced from".into(),
        );
    };
    let leaf = *tuning.exit_emplacement_tu;
    if leaf == 0 {
        return Err(
            "the shipped exit_emplacement_tu must be a positive cost, or zeroing it changes \
             nothing and the two quotes are equal for the wrong reason"
                .into(),
        );
    }
    Ok(leaf)
}

/// Rewrite the exit leaf in the live world, with no request in flight.
fn set_exit_tu(app: &mut App, leaf: u8) {
    let world = app.world_mut();
    let Some(mut tuning) = world.get_resource_mut::<CombatTuning>() else {
        return;
    };
    tuning.exit_emplacement_tu = ExitEmplacementTu::new(leaf);
}

/// What the sim's own pool holds for this ganger right now.
fn tu_of(app: &App, ganger: bevy::ecs::entity::Entity) -> Option<Tu> {
    app.world().get_entity(ganger).ok()?.get::<Tu>().copied()
}

/// A cell two clear steps off the seat, so a surcharge charged once reads apart from one per step.
fn two_steps_off(app: &App, seat: CellLevel) -> Result<CellLevelNet, TestError> {
    two_steps_from(app, CellLevelNet::from_sim(seat)).ok_or_else(|| NO_ROOM.into())
}

/// The `battle.cost` request that prices this actor's walk to `dest`.
fn move_cost_call(
    actor: bevy::ecs::entity::Entity,
    dest: CellLevelNet,
) -> Result<QaRequest, TestError> {
    cost_call(token_of(actor), &CostActNet::Move { dest })
        .ok_or_else(|| "a Move cost act must serialize to compact RON".into())
}

/// The TU one reply quoted, or a failure naming what it answered instead.
fn quoted(label: &str, body: &CostBody) -> Result<TuNet, TestError> {
    match (body.cost, body.refusal) {
        (Some(cost), None) => Ok(cost),
        (cost, refusal) => Err(format!(
            "the {label} quote must carry a real cost and no refusal, or two refusals both decode \
             to cost None and compare equal: cost {cost:?}, refusal {refusal:?}"
        )
        .into()),
    }
}

#[test]
fn battle_cost_quotes_the_exit_leaf_a_mounted_walk_is_charged() -> TestResult {
    let (mut app, port, scene) = manned_emplacement_under_the_shooter()?;
    let leaf = authored_exit_tu(&app)?;
    let call = move_cost_call(scene.shooter, two_steps_off(&app, scene.seat)?)?;
    let mut client = greet(&mut app, port)?;

    let charged = cost_body(client.exchange(&mut app, &call)?)?;
    set_exit_tu(&mut app, 0);
    let free = cost_body(client.exchange(&mut app, &call)?)?;

    let charged_tu = quoted("charged", &charged)?;
    let free_tu = quoted("baseline", &free)?;
    assert_eq!(
        u32::from(*charged_tu) - u32::from(*free_tu),
        u32::from(leaf),
        "`battle.cost` must add the exit leaf to a mounted walk: it quoted {} with the leaf at \
         {leaf} and {} with it zeroed, so it is pricing the route alone",
        *charged_tu,
        *free_tu,
    );
    Ok(())
}

#[test]
fn what_battle_cost_quotes_a_mounted_walk_is_what_the_walk_spends() -> TestResult {
    let (mut app, port, scene) = manned_emplacement_under_the_shooter()?;
    let dest = two_steps_off(&app, scene.seat)?;
    let call = move_cost_call(scene.shooter, dest)?;
    let mut client = greet(&mut app, port)?;

    let quote = quoted("walk", &cost_body(client.exchange(&mut app, &call)?)?)?;
    let Some(before) = tu_of(&app, scene.shooter) else {
        return Err("the mounted shooter must carry a Tu pool before it walks".into());
    };

    assert_caught_up(client.exchange(&mut app, &caught_up())?)?;
    client.exchange(
        &mut app,
        &run(
            ACT_SELECT,
            &ganger_argument(scene.shooter),
            RunOptions::default(),
        ),
    )?;
    assert_caught_up(client.exchange(&mut app, &caught_up())?)?;
    let moved = client.exchange(
        &mut app,
        &run(ACT_MOVE, &cell_argument(dest), RunOptions::default()),
    )?;
    accepted(ACT_MOVE, moved)?;
    assert_waited_for(
        WaitConditionNet::WalkComplete,
        client.exchange(&mut app, &walk_complete())?,
    )?;
    assert_caught_up(client.exchange(&mut app, &caught_up())?)?;
    let roster = decode::<RosterBody>(
        BATTLE_ROSTER,
        client.exchange(&mut app, &run(BATTLE_ROSTER, "()", RunOptions::default()))?,
    )?;

    let token = token_of(scene.shooter);
    let Some(card) = card_of(&roster, token) else {
        return Err(format!("the ganger that walked must be on the roster: {roster:?}").into());
    };
    assert_eq!(
        card.at, dest,
        "a walk cut short spends less than it was quoted for a reason that is not the surcharge, \
         so the walk must have reached {dest:?}; it ended on {:?}",
        card.at,
    );
    let Some(after) = tu_of(&app, scene.shooter) else {
        return Err("the shooter must still carry a Tu pool after it walks".into());
    };
    assert_eq!(
        u32::from(*before) - u32::from(*after),
        u32::from(*quote),
        "the walk must spend exactly what `battle.cost` quoted: it lost {} TU against a quote of \
         {} — the {WAIT} on WalkComplete says the route is finished",
        u32::from(*before) - u32::from(*after),
        *quote,
    );
    Ok(())
}
