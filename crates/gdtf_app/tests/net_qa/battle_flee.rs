use bevy::{app::App, state::state::State};
use gdtf_app::test_support::{BattleRunningComplete, BattleScapeState};
use gdtf_qa_protocol::{
    command::{CommandOutcome, RunOptions, UnavailableCode},
    message::QaResponse,
};

use super::{
    battle_fixture::{
        DRIVE_BUDGET, drive_into_battle_running, menu_app_with_net_qa, run_request, send,
    },
    command_exchange::{BATTLE_FLEE, exchange, run},
    socket_support::{TestResult, game_app_listening},
};

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

#[test]
fn battle_flee_ends_the_running_battle_the_way_the_flee_button_does() {
    let (mut app, tx) = menu_app_with_net_qa();
    drive_into_battle_running(&mut app);
    assert!(
        !app.world().contains_resource::<BattleRunningComplete>(),
        "the battle must still be running, or this case proves nothing about fleeing",
    );

    let pending = send(&tx, run_request(BATTLE_FLEE, "()"));
    app.update();
    assert!(
        app.world().contains_resource::<BattleRunningComplete>(),
        "flee must go through the same insert the Flee button makes, on the frame it is claimed",
    );

    let mut answered = None;
    for _ in 0..DRIVE_BUDGET {
        app.update();
        if let Ok(reply) = pending.try_recv() {
            answered = Some(reply);
            break;
        }
    }

    let Some(QaResponse::Outcome(CommandOutcome::Ran { reply, .. })) = answered else {
        unreachable!(
            "battle.flee must answer once the battle has left its running phase, got {answered:?}"
        );
    };
    assert_ne!(
        battlescape_state(&app),
        Some(BattleScapeState::BattleRunning),
        "the battle must have left BattleRunning, not merely been marked complete",
    );
    assert!(
        !reply.as_str().contains("battlescape:Some(BattleRunning)"),
        "the reply reports where the app landed, which is no longer the running battle: {}",
        reply.as_str(),
    );
}

#[test]
fn battle_flee_from_the_menu_is_refused_rather_than_parked() -> TestResult {
    let reply = exchange(
        game_app_listening,
        run(BATTLE_FLEE, "()", RunOptions::default()),
    )?;
    let QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) = reply else {
        unreachable!("with no battle running there is nothing to flee, got {reply:?}");
    };
    assert_eq!(code, UnavailableCode::WrongState);
    assert!(
        note.as_str().contains("running"),
        "the refusal names the precondition that is missing: {}",
        note.as_str(),
    );
    Ok(())
}
