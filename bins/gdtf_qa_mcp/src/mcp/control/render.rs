//! Turning a launch or stop outcome into the MCP content block a caller reads.

use serde_json::{Value, json};

use crate::{
    lifecycle::{LaunchFailure, LaunchOutcome, LaunchSpec, StopOutcome},
    mcp::content::{text_content, tool_error},
};

/// Render a launch outcome as an MCP content block.
///
/// EVERY non-error outcome reports the recipe of the child the caller is about to drive,
/// alongside the port and pid: which package, which features, and — resolved, never left
/// implicit — which directory the launcher built in. That is what tells a caller WHICH
/// CHECKOUT is under test instead of leaving it to be assumed, and it has to hold for
/// `already_running` as much as for a fresh launch: the reply names the recipe of the
/// child that is actually up, which is not necessarily the one this call asked for
/// (GTW-875).
pub(super) fn render_launch(outcome: &LaunchOutcome, requested: &LaunchSpec) -> Value {
    match outcome {
        LaunchOutcome::Launched { port, pid } => text_content(&json!({
            "status": "launched", "port": **port, "pid": **pid,
            "package": requested.package().as_str(),
            "features": requested.features().render(),
            "working_dir": resolved_working_dir(requested),
        })),
        LaunchOutcome::AlreadyRunning { port, pid, recipe } => text_content(&json!({
            "status": "already_running", "port": **port, "pid": **pid,
            "package": recipe.package().as_str(),
            "features": recipe.features().render(),
            "working_dir": resolved_working_dir(recipe),
        })),
        LaunchOutcome::Failed(failure) => tool_error(&launch_failure_message(failure, requested)),
    }
}

/// The directory the launcher actually ran in: the recipe's, or the MCP host's own when
/// the recipe named none.
fn resolved_working_dir(spec: &LaunchSpec) -> String {
    spec.resolved_working_dir().map_or_else(
        || "<unknown>".to_owned(),
        |dir| dir.to_string_lossy().into_owned(),
    )
}

/// A human-readable message for a launch failure, carrying the child's stderr tail — or,
/// for a recipe mismatch, both the running recipe and the `requested` one.
fn launch_failure_message(failure: &LaunchFailure, requested: &LaunchSpec) -> String {
    match failure {
        LaunchFailure::Spawn(reason) => {
            format!("could not launch the game: {}", reason.as_str())
        }
        LaunchFailure::RecipeMismatch(running) => format!(
            "a game is already running from a different recipe, and nothing was launched. \
             running: package {}, features {}, working_dir {}. requested: package {}, \
             features {}, working_dir {}. Call stop_game first if you want the requested \
             build.",
            running.package().as_str(),
            running
                .features()
                .render()
                .unwrap_or_else(|| "none".to_owned()),
            resolved_working_dir(running),
            requested.package().as_str(),
            requested
                .features()
                .render()
                .unwrap_or_else(|| "none".to_owned()),
            resolved_working_dir(requested),
        ),
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
pub(super) fn render_stop(outcome: &StopOutcome) -> Value {
    match outcome {
        StopOutcome::Stopped { pid } => text_content(&json!({ "status": "stopped", "pid": **pid })),
        StopOutcome::NotRunning => text_content(&json!({ "status": "not_running" })),
    }
}
