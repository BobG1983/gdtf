//! A live app whose screen is behind the act log refuses every act with `Replaying`.

use bevy::app::App;
use cobalt_mcp_protocol::{
    command::{CommandOutcome, RunOptions, UnavailableCode},
    message::McpResponse,
    ports::McpPort,
};

use super::{
    battle_reads::{an_unreachable_cell, cell_argument},
    battle_setup::hold_the_screen_still,
    command_exchange::{
        ACT_ENTER_EMPLACEMENT, ACT_EXECUTE, ACT_EXIT_EMPLACEMENT, ACT_MELEE, ACT_MOVE,
        ACT_OPEN_DOOR, ACT_SELECT_NEXT, ACT_SET_STANCE, ACT_SHOVE, ACT_STABILIZE,
        ACT_THROW_GRENADE, exchange_all, run,
    },
    socket_support::{TestError, TestResult, battle_app_listening},
};

/// A running battle whose screen has been stopped, so the gate the act bus reads is shut.
fn battle_still_replaying() -> Result<(App, McpPort), TestError> {
    let (mut app, port) = battle_app_listening()?;
    hold_the_screen_still(&mut app)?;
    Ok((app, port))
}

/// The refusal code a reply carries, or a failure saying what came back instead.
fn refusal(name: &str, reply: McpResponse) -> Result<UnavailableCode, TestError> {
    match reply {
        McpResponse::Outcome(CommandOutcome::Unavailable { code, .. }) => Ok(code),
        other => {
            Err(format!("`{name}` must refuse while the screen is behind, got {other:?}").into())
        }
    }
}

#[test]
fn an_act_is_refused_replaying_while_the_screen_is_behind_the_log() -> TestResult {
    let asked = [
        (ACT_SELECT_NEXT, "()".to_owned()),
        (ACT_MOVE, cell_argument(an_unreachable_cell())),
        (ACT_SET_STANCE, "(stance:Prone)".to_owned()),
        (ACT_MELEE, "()".to_owned()),
        (ACT_SHOVE, "()".to_owned()),
        (ACT_STABILIZE, "()".to_owned()),
        (ACT_EXECUTE, "()".to_owned()),
        (ACT_THROW_GRENADE, "()".to_owned()),
        (ACT_OPEN_DOOR, "()".to_owned()),
        (ACT_ENTER_EMPLACEMENT, "()".to_owned()),
        (ACT_EXIT_EMPLACEMENT, "()".to_owned()),
    ];
    let replies = exchange_all(
        battle_still_replaying,
        asked
            .iter()
            .map(|(name, arguments)| run(name, arguments.as_str(), RunOptions::default()))
            .collect(),
    )?;
    assert_eq!(
        replies.len(),
        asked.len(),
        "every act asked for must be answered, or the case proves nothing",
    );
    for ((name, _), reply) in asked.iter().zip(replies) {
        assert_eq!(
            refusal(name, reply)?,
            UnavailableCode::Replaying,
            "`{name}` may not act while the screen is still playing the log back — the act bus \
             drops an intent whose gate is shut, so the call has to refuse instead of being \
             swallowed",
        );
    }
    Ok(())
}
