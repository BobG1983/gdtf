//! The two host-local entry points — [`handle_launch`] and [`handle_stop`] — and the
//! `port` argument they read.

use serde_json::Value;

use super::render::{render_launch, render_stop};
use crate::{
    game::{GameLink, GamePort},
    lifecycle::{GameLifecycle, LaunchOutcome},
    mcp::{call::ToolCallOutcome, launch_args::parse_launch_spec},
};

/// Handle a `launch_game` call: read the launch recipe, ensure a game is running on the
/// chosen port, re-point the game link at it, and render the outcome.
#[must_use]
pub fn handle_launch(
    args: &Value,
    game: &mut dyn GameLink,
    lifecycle: &mut dyn GameLifecycle,
) -> ToolCallOutcome {
    let port = match parse_port(args) {
        Ok(port) => port,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    let spec = match parse_launch_spec(args) {
        Ok(spec) => spec,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    let outcome = lifecycle.launch(port, &spec);
    // Point the forwarding link at whichever port the game is actually on, so subsequent
    // tool calls reach the child this launch ensured.
    match &outcome {
        LaunchOutcome::Launched { port, .. } | LaunchOutcome::AlreadyRunning { port, .. } => {
            game.retarget(*port);
        }
        LaunchOutcome::Failed(_) => {}
    }
    ToolCallOutcome::Result(render_launch(&outcome, &spec))
}

/// Handle a `stop_game` call: stop the running child (if any) and render the outcome.
#[must_use]
pub fn handle_stop(lifecycle: &mut dyn GameLifecycle) -> ToolCallOutcome {
    ToolCallOutcome::Result(render_stop(&lifecycle.stop()))
}

/// Parse the optional `launch_game` `port` argument, defaulting to the environment port.
pub(super) fn parse_port(args: &Value) -> Result<GamePort, String> {
    match args.get("port") {
        None | Some(Value::Null) => Ok(GamePort::from_env()),
        Some(value) => {
            let Some(raw) = value.as_u64() else {
                return Err("`port` must be a non-negative integer".to_owned());
            };
            let Ok(narrow) = u16::try_from(raw) else {
                return Err("`port` must be in the range 0..=65535".to_owned());
            };
            Ok(GamePort::new(narrow))
        }
    }
}
