//! The `launch_game` / `stop_game` tool handlers — starting and stopping the game process
//! (GTW-745).
//!
//! These two tools are host-local: they do NOT forward a `QaRequest` to a running game
//! the way the other seven do — they start and stop the game process itself through the
//! [`GameLifecycle`]. On a successful launch the game link is re-pointed at the port the
//! child bound, so the following forwarding calls reach it. Every path renders a normal
//! MCP content block: a launch or stop failure is a tool error, never a crash.

use serde_json::{Value, json};

use super::{
    call::ToolCallOutcome,
    content::{text_content, tool_error},
};
use crate::{
    game::{GameLink, GamePort},
    lifecycle::{GameLifecycle, LaunchFailure, LaunchOutcome, StopOutcome},
};

/// Handle a `launch_game` call: ensure a game is running on the chosen port, re-point the
/// game link at it, and render the outcome.
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
    let outcome = lifecycle.launch(port);
    // Point the forwarding link at whichever port the game is actually on, so subsequent
    // tool calls reach the child this launch ensured.
    match &outcome {
        LaunchOutcome::Launched { port, .. } | LaunchOutcome::AlreadyRunning { port, .. } => {
            game.retarget(*port);
        }
        LaunchOutcome::Failed(_) => {}
    }
    ToolCallOutcome::Result(render_launch(&outcome))
}

/// Handle a `stop_game` call: stop the running child (if any) and render the outcome.
#[must_use]
pub fn handle_stop(lifecycle: &mut dyn GameLifecycle) -> ToolCallOutcome {
    ToolCallOutcome::Result(render_stop(&lifecycle.stop()))
}

/// Parse the optional `launch_game` `port` argument, defaulting to the environment port.
fn parse_port(args: &Value) -> Result<GamePort, String> {
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

/// Render a launch outcome as an MCP content block.
fn render_launch(outcome: &LaunchOutcome) -> Value {
    match outcome {
        LaunchOutcome::Launched { port, pid } => text_content(&json!({
            "status": "launched", "port": **port, "pid": **pid,
        })),
        LaunchOutcome::AlreadyRunning { port, pid } => text_content(&json!({
            "status": "already_running", "port": **port, "pid": **pid,
        })),
        LaunchOutcome::Failed(failure) => tool_error(&launch_failure_message(failure)),
    }
}

/// A human-readable message for a launch failure, carrying the child's stderr tail.
fn launch_failure_message(failure: &LaunchFailure) -> String {
    match failure {
        LaunchFailure::Spawn(reason) => {
            format!("could not launch the game: {}", reason.as_str())
        }
        LaunchFailure::Timeout(tail) => format!(
            "the game did not become ready before the timeout and was stopped. \
             stderr tail:\n{}",
            tail.as_str()
        ),
        LaunchFailure::ExitedEarly(tail) => format!(
            "the game exited before it became ready. stderr tail:\n{}",
            tail.as_str()
        ),
    }
}

/// Render a stop outcome as an MCP content block.
fn render_stop(outcome: &StopOutcome) -> Value {
    match outcome {
        StopOutcome::Stopped { pid } => text_content(&json!({ "status": "stopped", "pid": **pid })),
        StopOutcome::NotRunning => text_content(&json!({ "status": "not_running" })),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_qa_protocol::envelope::{QaRequest, QaResponse};
    use serde_json::json;

    use super::{
        GameLifecycle, GameLink, GamePort, LaunchFailure, LaunchOutcome, StopOutcome,
        ToolCallOutcome, handle_launch, handle_stop, parse_port, render_launch, render_stop,
    };
    use crate::{
        error::McpError,
        lifecycle::{ChildPid, StderrTail},
    };

    /// A lifecycle whose launch / stop return canned outcomes — no real processes.
    struct StubLifecycle {
        /// The outcome `launch` returns.
        launch: LaunchOutcome,
        /// The outcome `stop` returns.
        stop:   StopOutcome,
    }

    impl GameLifecycle for StubLifecycle {
        fn launch(&mut self, _port: GamePort) -> LaunchOutcome {
            self.launch.clone()
        }

        fn stop(&mut self) -> StopOutcome {
            self.stop.clone()
        }
    }

    /// A game link that records the last port it was re-pointed at.
    struct RecordingLink {
        /// The port the last `retarget` set, if any.
        retargeted: Option<GamePort>,
    }

    impl GameLink for RecordingLink {
        fn request(&mut self, _request: QaRequest) -> Result<QaResponse, McpError> {
            Err(McpError::Disconnected)
        }

        fn retarget(&mut self, port: GamePort) {
            self.retargeted = Some(port);
        }
    }

    /// An explicit `port` argument parses to that port; a too-large one is rejected.
    #[test]
    fn parse_port_reads_explicit_and_rejects_out_of_range() {
        let Ok(port) = parse_port(&json!({ "port": 4321 })) else {
            unreachable!("an in-range port parses");
        };
        assert_eq!(*port, 4321);
        assert!(parse_port(&json!({ "port": 99999 })).is_err());
        assert!(parse_port(&json!({ "port": "nope" })).is_err());
    }

    /// A successful launch re-points the game link at the launched port and renders a
    /// non-error block.
    #[test]
    fn handle_launch_retargets_link_and_renders_launched() {
        let mut lifecycle = StubLifecycle {
            launch: LaunchOutcome::Launched {
                port: GamePort::new(4321),
                pid:  ChildPid::new(999),
            },
            stop:   StopOutcome::NotRunning,
        };
        let mut link = RecordingLink { retargeted: None };
        let ToolCallOutcome::Result(result) = handle_launch(&json!({}), &mut link, &mut lifecycle)
        else {
            unreachable!("a launch renders a result block");
        };
        assert_eq!(link.retargeted, Some(GamePort::new(4321)));
        assert_eq!(result["isError"], json!(false));
        let Some(text) = result["content"][0]["text"].as_str() else {
            unreachable!("the launch reply is text content");
        };
        assert!(text.contains("launched"), "rendered: {text}");
        assert!(text.contains("4321"), "rendered: {text}");
    }

    /// A boot-timeout failure renders as a tool error carrying the stderr tail.
    #[test]
    fn render_launch_timeout_is_tool_error_with_tail() {
        let rendered = render_launch(&LaunchOutcome::Failed(LaunchFailure::Timeout(
            StderrTail::new("panicked at boot".to_owned()),
        )));
        assert_eq!(rendered["isError"], json!(true));
        let Some(text) = rendered["content"][0]["text"].as_str() else {
            unreachable!("a tool error carries a text reason");
        };
        assert!(text.contains("panicked at boot"), "rendered: {text}");
    }

    /// Stopping with nothing running renders a typed non-error `not_running` result.
    #[test]
    fn handle_stop_not_running_is_typed_result() {
        let mut lifecycle = StubLifecycle {
            launch: LaunchOutcome::Failed(LaunchFailure::Spawn(crate::lifecycle::SpawnError::new(
                "unused".to_owned(),
            ))),
            stop:   StopOutcome::NotRunning,
        };
        let ToolCallOutcome::Result(result) = handle_stop(&mut lifecycle) else {
            unreachable!("a stop renders a result block");
        };
        assert_eq!(result["isError"], json!(false));
        assert_eq!(
            render_stop(&StopOutcome::NotRunning)["isError"],
            json!(false)
        );
        let Some(text) = result["content"][0]["text"].as_str() else {
            unreachable!("the stop reply is text content");
        };
        assert!(text.contains("not_running"), "rendered: {text}");
    }
}
