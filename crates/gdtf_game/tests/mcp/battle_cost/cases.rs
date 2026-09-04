//! `battle.cost` answered over the real socket, in a live battlescape.

use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_protocol::{
    command::{CommandOutcome, RefusalNote, UnavailableCode},
    message::QaResponse,
};
use gdtf_battle_sim::{
    battle::TeardownBattleRequested, ganger::Tu, prelude::CellLevel, weapon::ModeKind,
};
use gdtf_game::qa_wire::{
    act_payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet},
    cell::CellLevelNet,
    cost::{CostActNet, CostRefusalNet},
    misc::ModeKindNet,
    token::{DoorToken, EmplacementToken, GangerToken},
    vitals::TuNet,
};

use super::support::{
    Pose, a_reachable_cell, cost_body, cost_call, cost_calls, first_body, pose, route_cost, settle,
};
use crate::{
    battle_reads::a_player_ganger,
    command_exchange::{exchange, exchange_in_battle},
    socket_support::{TestError, TestResult, game_app_listening},
};

/// A token that names nothing, so a target the case does not have still asks a real question.
const NOTHING: u64 = 0;

/// How many acts a case asked about.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct ActCount(usize);

impl ActCount {
    const fn new(asked: usize) -> Self {
        Self(asked)
    }
}

/// Every act the command prices, so a case can walk the whole list.
fn every_act(actor: Entity, at: CellLevelNet) -> Vec<CostActNet> {
    let token = GangerToken::new(actor.to_bits());
    vec![
        CostActNet::Move { dest: at },
        CostActNet::Fire {
            target: at,
            mode:   ModeKindNet::from_sim(ModeKind::Single),
        },
        CostActNet::Reload,
        CostActNet::SetStance {
            stance: StanceNet::Prone,
        },
        CostActNet::SetFacing {
            facing: FacingNet::South,
        },
        CostActNet::SetAiming {
            aim: AimNet::new(true),
        },
        CostActNet::Shove { target: token },
        CostActNet::OpenDoor {
            target: DoorToken::new(NOTHING),
        },
        CostActNet::EnterEmplacement {
            target: EmplacementToken::new(NOTHING),
        },
        CostActNet::ExitEmplacement {
            target: EmplacementToken::new(NOTHING),
        },
        CostActNet::ThrowGrenade { target: at },
        CostActNet::Melee {
            target: MeleeTargetNet::Ganger(token),
        },
    ]
}

#[test]
fn an_affordable_move_quotes_the_route_the_pathfinder_costs() -> TestResult {
    let mut planned: Option<(Entity, CellLevel)> = None;
    let (app, replies) = exchange_in_battle(|app| {
        let Some(walk) = a_planned_walk(app) else {
            return Vec::new();
        };
        planned = Some(walk);
        cost_calls(
            walk.0,
            &[CostActNet::Move {
                dest: CellLevelNet::from_sim(walk.1),
            }],
        )
    })?;
    let (actor, dest) = a_walk(planned)?;
    let body = first_body(replies)?;
    let Some(expected) = route_cost(&app, actor, dest) else {
        return Err("the same world must still cost the route the reply quoted".into());
    };
    assert_eq!(
        body.cost,
        Some(TuNet::new(*expected)),
        "the quote must be the total the sim's own pathfinder charges for that route",
    );
    assert!(*body.legal, "a route the pool covers is legal: {body:?}");
    assert_eq!(
        body.refusal, None,
        "a route the pool covers carries no refusal: {body:?}",
    );
    Ok(())
}

#[test]
fn a_move_the_pool_cannot_pay_for_is_refused_and_still_quoted() -> TestResult {
    let mut planned: Option<(Entity, CellLevel)> = None;
    let (app, replies) = exchange_in_battle(|app| {
        let Some(walk) = a_planned_walk(app) else {
            return Vec::new();
        };
        planned = Some(walk);
        let calls = cost_calls(
            walk.0,
            &[CostActNet::Move {
                dest: CellLevelNet::from_sim(walk.1),
            }],
        );
        if let Ok(mut row) = app.world_mut().get_entity_mut(walk.0) {
            row.insert(Tu::new(0));
        }
        calls
    })?;
    let (actor, dest) = a_walk(planned)?;
    let body = first_body(replies)?;
    let Some(expected) = route_cost(&app, actor, dest) else {
        return Err("the same world must still cost the route the reply quoted".into());
    };
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::CannotAfford),
        "an empty pool must be refused by the sim's own affordability check: {body:?}",
    );
    assert!(!*body.legal, "a refused act is not legal: {body:?}");
    assert_eq!(
        body.cost,
        Some(TuNet::new(*expected)),
        "the price stands even when the pool cannot pay it",
    );
    Ok(())
}

#[test]
fn pricing_every_act_leaves_the_battle_exactly_as_it_was() -> TestResult {
    let mut planned: Option<(Entity, Pose, ActCount)> = None;
    let (app, replies) = exchange_in_battle(|app| {
        settle(app);
        let Some((actor, at)) = a_player_ganger(app) else {
            return Vec::new();
        };
        let Some(before) = pose(app, actor) else {
            return Vec::new();
        };
        let acts = every_act(actor, at);
        planned = Some((actor, before, ActCount::new(acts.len())));
        cost_calls(actor, &acts)
    })?;
    let Some((actor, before, asked)) = planned else {
        return Err("the battle fixture must field a player ganger to price acts for".into());
    };
    assert_eq!(
        replies.len(),
        *asked,
        "every act in the list must come back with its own reply",
    );
    for reply in replies {
        let _body = cost_body(reply)?;
    }
    let Some(after) = pose(&app, actor) else {
        return Err("the ganger the case priced acts for must still be there".into());
    };
    assert_eq!(
        after, before,
        "battle.cost only reads: the actor's pose and the selection resources must be untouched",
    );
    Ok(())
}

#[test]
fn a_cost_call_outside_a_running_battle_is_refused_with_the_state_it_needs() -> TestResult {
    let Some(request) = cost_call(GangerToken::new(NOTHING), &CostActNet::Reload) else {
        return Err("a Reload cost call must encode".into());
    };
    let (code, note) = refusal(exchange(game_app_listening, request)?)?;
    assert_eq!(
        code,
        UnavailableCode::WrongState,
        "a host in the wrong state refuses WrongState — {note:?}",
    );
    assert!(
        note.as_str().contains("running"),
        "the note must name the state the call needs: {note:?}",
    );
    Ok(())
}

#[test]
fn a_cost_call_in_a_running_battle_whose_sim_state_is_gone_names_the_missing_model() -> TestResult {
    let (_app, replies) = exchange_in_battle(|app| {
        settle(app);
        app.world_mut().write_message(TeardownBattleRequested);
        settle(app);
        cost_call(GangerToken::new(NOTHING), &CostActNet::Reload)
            .map(|request| vec![request])
            .unwrap_or_default()
    })?;
    let Some(reply) = replies.into_iter().next() else {
        return Err("a Reload cost call must come back with a reply".into());
    };
    let (code, note) = refusal(reply)?;
    assert_eq!(
        code,
        UnavailableCode::MissingModel,
        "the battle is still in its running phase, so what is missing is the sim state the price \
         is read from, not the host's state — {note:?}",
    );
    assert!(
        note.as_str().contains("not loaded"),
        "the note has to say the sim state is not there, or a caller cannot tell a price of zero \
         from a price nobody could compute: {note:?}",
    );
    Ok(())
}

/// The code and note of a refused call, or a failure naming what came back instead.
fn refusal(reply: QaResponse) -> Result<(UnavailableCode, RefusalNote), TestError> {
    match reply {
        QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) => Ok((code, note)),
        other => Err(format!("battle.cost must refuse this call, got {other:?}").into()),
    }
}

/// Settle the app, then pick a player ganger and somewhere it can actually walk to.
fn a_planned_walk(app: &mut App) -> Option<(Entity, CellLevel)> {
    settle(app);
    let (actor, _) = a_player_ganger(app)?;
    let dest = a_reachable_cell(app, actor)?;
    Some((actor, dest))
}

fn a_walk(planned: Option<(Entity, CellLevel)>) -> Result<(Entity, CellLevel), TestError> {
    planned.ok_or_else(|| {
        "the battle fixture must field a player ganger with somewhere to walk to".into()
    })
}
