use std::sync::mpsc::Sender;

use bevy::app::App;
use cobalt_mcp_host::IncomingRequest;
use cobalt_mcp_protocol::{
    command::{CommandOutcome, RunOptions, UnavailableCode},
    message::McpResponse,
};
use gdtf_game::{
    qa_wire::misc::SeedNet,
    test_support::{GenerationComplete, ResolvedBattleSeed},
};
use serde::Deserialize;

use super::{
    battle_fixture::{menu_app_with_mcp, run_request, send},
    command_exchange::{APP_PHASE, BATTLE_START, exchange_all, run},
    socket_support::{TestResult, game_app_listening},
};

/// A seed no wall clock will ever produce, so a verbatim reply cannot be a coincidence.
const PINNED_SEED: u64 = 0x0BAD_F00D_DEAD_BEEF;

#[derive(Debug, Deserialize)]
struct StartBody {
    seed: SeedNet,
}

struct StartedBattle {
    app:   App,
    tx:    Sender<IncomingRequest>,
    reply: McpResponse,
}

fn started_seed(reply: &McpResponse) -> SeedNet {
    let McpResponse::Outcome(CommandOutcome::Ran { reply, .. }) = reply else {
        unreachable!("battle.start must RUN once generation finishes, got {reply:?}");
    };
    let body = reply.as_str();
    let Ok(started) = ron::de::from_str::<StartBody>(body) else {
        unreachable!("the reply body decodes into the published start shape: {body}");
    };
    started.seed
}

fn start_over_the_channel(arguments: &str) -> StartedBattle {
    let (mut app, tx) = menu_app_with_mcp();
    let pending = send(&tx, run_request(BATTLE_START, arguments));
    let reply = loop {
        app.update();
        if let Ok(reply) = pending.try_recv() {
            break reply;
        }
    };
    StartedBattle { app, tx, reply }
}

fn resolved_seed(app: &App) -> SeedNet {
    let Some(resolved) = app.world().get_resource::<ResolvedBattleSeed>() else {
        unreachable!("a started battle must have recorded the root seed it resolved");
    };
    SeedNet::new(***resolved)
}

#[test]
fn battle_start_reports_the_seed_the_battle_actually_recorded() {
    let started = start_over_the_channel("()");
    assert_eq!(
        started_seed(&started.reply),
        resolved_seed(&started.app),
        "with no seed given the game resolves its own, and the reply must report THAT value — \
         the one procgen was handed, not a fresh roll",
    );
}

#[test]
fn a_pinned_seed_comes_back_verbatim() {
    let started = start_over_the_channel(&format!("(seed:Some({PINNED_SEED}))"));
    assert_eq!(
        started_seed(&started.reply),
        SeedNet::new(PINNED_SEED),
        "a caller that pins the seed must get that exact seed back",
    );
    assert_eq!(
        resolved_seed(&started.app),
        SeedNet::new(PINNED_SEED),
        "and the battle must have been generated from it, not merely told about it",
    );
}

#[test]
fn battle_start_holds_its_reply_until_generation_has_finished() {
    let (mut app, tx) = menu_app_with_mcp();
    let pending = send(&tx, run_request(BATTLE_START, "()"));

    let generation_had_finished = loop {
        let generated = app.world().contains_resource::<GenerationComplete>();
        app.update();
        if pending.try_recv().is_ok() {
            break generated;
        }
    };

    assert!(
        generation_had_finished,
        "battle.start answered on a frame that opened with no GenerationComplete — the seed is \
         recorded when generation STARTS, so a reply that goes out on the seed alone hands the \
         caller a battle still being built",
    );
}

#[test]
fn a_second_start_from_inside_the_battle_is_refused() {
    let StartedBattle { mut app, tx, .. } = start_over_the_channel("()");
    let pending = send(&tx, run_request(BATTLE_START, "()"));
    let answered = loop {
        app.update();
        if let Ok(reply) = pending.try_recv() {
            break reply;
        }
    };
    let McpResponse::Outcome(CommandOutcome::Unavailable { code, .. }) = answered else {
        unreachable!("a start from inside a battle must be refused, not parked; got {answered:?}");
    };
    assert_eq!(
        code,
        UnavailableCode::WrongState,
        "battle.start starts a battle from the menu, and the app has left it",
    );
}

#[test]
fn battle_start_over_the_socket_leaves_the_app_in_the_battlescape() -> TestResult {
    let mut replies = exchange_all(
        game_app_listening,
        vec![
            run(
                BATTLE_START,
                &format!("(seed:Some({PINNED_SEED}))"),
                RunOptions::default(),
            ),
            run(APP_PHASE, "()", RunOptions::default()),
        ],
    )?;
    let Some(phase) = replies.pop() else {
        unreachable!("two requests yield two replies");
    };
    let Some(started) = replies.pop() else {
        unreachable!("two requests yield two replies");
    };
    assert_eq!(
        started_seed(&started),
        SeedNet::new(PINNED_SEED),
        "over the real socket too, a caller that pins the seed gets that exact seed back",
    );

    let McpResponse::Outcome(CommandOutcome::Ran { reply, .. }) = phase else {
        unreachable!("a plain app.phase call must RUN, got {phase:?}");
    };
    assert!(
        reply.as_str().contains("game:Some(BattleScape)"),
        "once battle.start has answered the app is in the battle, not the menu: {}",
        reply.as_str(),
    );
    Ok(())
}
