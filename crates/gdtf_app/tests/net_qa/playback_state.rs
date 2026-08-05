use bevy::app::App;
use gdtf_app::qa_wire::shell::CaughtUpNet;
use gdtf_battle_presenter::playback::{ActHold, PlaybackCursor};
use gdtf_battle_sim::act_log::ActLog;
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{
    command::{CommandOutcome, RunOptions},
    message::QaResponse,
};
use serde::Deserialize;

use super::{
    command_exchange::{PLAYBACK_STATE, exchange, run},
    socket_support::{TestError, TestResult, battle_app_listening, game_app_listening},
};

/// Longer than any exchange can drive, so the hold cannot expire before the reply.
const UNENDING_HOLD_SECONDS: f32 = 3600.0;

#[derive(Debug, Deserialize)]
struct PlaybackBody {
    caught_up: CaughtUpNet,
}

/// A live battle whose cursor is holding, so the screen has not caught the act log.
fn playback_holding_app() -> Result<(App, NetQaPort), TestError> {
    let (mut app, port) = battle_app_listening()?;
    if !app.world().contains_resource::<ActLog>() {
        return Err("a running battle must carry an act log for the gate to close on".into());
    }
    app.world_mut()
        .resource_mut::<PlaybackCursor>()
        .hold_for(ActHold::timed(UNENDING_HOLD_SECONDS));
    Ok((app, port))
}

fn caught_up(reply: QaResponse) -> CaughtUpNet {
    let QaResponse::Outcome(CommandOutcome::Ran { reply, .. }) = reply else {
        unreachable!("a plain playback.state call must RUN, got {reply:?}");
    };
    let body = reply.as_str();
    let Ok(playback) = ron::de::from_str::<PlaybackBody>(body) else {
        unreachable!("the reply body decodes into the published playback shape: {body}");
    };
    playback.caught_up
}

#[test]
fn playback_state_is_caught_up_when_no_battle_is_running() -> TestResult {
    let reply = exchange(
        game_app_listening,
        run(PLAYBACK_STATE, "()", RunOptions::default()),
    )?;
    assert_eq!(
        caught_up(reply),
        CaughtUpNet::new(true),
        "in the Menu there is no cursor and no act log, so the gate is open",
    );
    Ok(())
}

#[test]
fn playback_state_reports_not_caught_up_while_the_cursor_holds() -> TestResult {
    let reply = exchange(
        playback_holding_app,
        run(PLAYBACK_STATE, "()", RunOptions::default()),
    )?;
    assert_eq!(
        caught_up(reply),
        CaughtUpNet::new(false),
        "the reply must carry the live gate, not a constant: this battle's cursor was holding \
         before the command ran, which is what keeps player input waiting",
    );
    Ok(())
}
