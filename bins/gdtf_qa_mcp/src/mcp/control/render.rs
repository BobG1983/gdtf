//! Turning a launch or stop outcome into the MCP content block a caller reads.

use serde_json::{Value, json};

use crate::{
    hosts::QaHost,
    lifecycle::{
        BootTimeout, LaunchFailure, LaunchOutcome, LaunchSpec, OrphanPid, StderrTail, StopOutcome,
    },
    link::QaPort,
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
pub(super) fn render_launch(
    host: QaHost,
    outcome: &LaunchOutcome,
    requested: &LaunchSpec,
) -> Value {
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
        LaunchOutcome::Failed(failure) => {
            tool_error(&launch_failure_message(host, failure, requested))
        }
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
fn launch_failure_message(host: QaHost, failure: &LaunchFailure, requested: &LaunchSpec) -> String {
    let label = host.label();
    match failure {
        LaunchFailure::Spawn(reason) => {
            format!("could not launch the {label}: {}", reason.as_str())
        }
        LaunchFailure::RecipeMismatch(running) => format!(
            "a {label} is already running from a different recipe, and nothing was launched. \
             running: package {}, features {}, working_dir {}. requested: package {}, \
             features {}, working_dir {}. Call {} first if you want the requested \
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
            host.stop_tool_name(),
        ),
        LaunchFailure::PortHeldByOrphan { port, pid } => format!(
            "the {label} port {} is already held by a process this MCP host did not start{} \
             — an orphan — so nothing was started. That is what an MCP host restart leaves \
             behind: the child outlives the host process that spawned it, so this process \
             owns no handle to it. Call {} to stop it, then try again.",
            **port,
            holder_clause(*pid),
            host.stop_tool_name(),
        ),
        LaunchFailure::Timeout { tail, waited } => timeout_message(host, *waited, requested, tail),
        LaunchFailure::ExitedEarly(tail) => format!(
            "the {label} exited before it became ready. stderr tail:\n{}",
            tail.as_str()
        ),
    }
}

/// The timeout message — it NAMES THE BUILD as the likely cause and gives the warm-up
/// command, rather than reporting a bare "timed out" (GTW-808 clause 7).
///
/// A launcher's wait covers two very different things: `cargo` compiling the package, and
/// the compiled binary booting. Only the first one takes minutes, and it is invisible in
/// the child's stderr tail unless the reader already knows to look for `Compiling` lines.
/// Saying so, with the exact command that removes the wait, is the difference between a
/// diagnosable failure and a mystery.
fn timeout_message(
    host: QaHost,
    waited: BootTimeout,
    requested: &LaunchSpec,
    tail: &StderrTail,
) -> String {
    let label = host.label();
    let features = requested
        .features()
        .render()
        .map_or_else(String::new, |list| format!(" --features {list}"));
    format!(
        "the {label} did not answer within {:?} and was stopped. The most likely cause is \
         the BUILD, not the app: this launch runs `cargo run -p {}{features}`, and a \
         recipe that has never been compiled in {} spends that whole wait compiling before \
         it can bind a port. Warm it first with `cargo build -p {}{features}` in that \
         directory and launch again; the stderr tail below shows how far the build got. \
         stderr tail:\n{}",
        *waited,
        requested.package().as_str(),
        resolved_working_dir(requested),
        requested.package().as_str(),
        tail.as_str()
    )
}

/// The " (process 43744)" clause naming an orphan's holder, or an empty string when the
/// operating system could not name it.
fn holder_clause(pid: OrphanPid) -> String {
    match pid {
        OrphanPid::Known(child) => format!(" (process {})", *child),
        OrphanPid::Unknown => String::new(),
    }
}

/// The `pid` field of an orphan reply — the number, or `null` when it could not be named.
fn holder_field(pid: OrphanPid) -> Value {
    match pid {
        OrphanPid::Known(child) => json!(*child),
        OrphanPid::Unknown => Value::Null,
    }
}

/// Render a stop outcome as an MCP content block.
///
/// The two orphan answers are reported AS orphans, never as `not_running` (GTW-926): a
/// caller told "not running" about a child that is alive and holding the port has no route
/// back to it and no reason to look for one. A stop that could not free the port is a tool
/// ERROR, because the caller's next step differs — it has to deal with a process this host
/// cannot signal.
pub(super) fn render_stop(host: QaHost, outcome: &StopOutcome) -> Value {
    match outcome {
        StopOutcome::Stopped { pid } => text_content(&json!({ "status": "stopped", "pid": **pid })),
        StopOutcome::OrphanStopped { port, pid } => text_content(&json!({
            "status": "orphan_stopped", "port": **port, "pid": holder_field(*pid),
        })),
        StopOutcome::OrphanHeld { port, pid } => {
            tool_error(&orphan_held_message(host, *port, *pid))
        }
        StopOutcome::NotRunning => text_content(&json!({ "status": "not_running" })),
    }
}

/// The message for an orphan that is still holding the port after the stop.
fn orphan_held_message(host: QaHost, port: QaPort, pid: OrphanPid) -> String {
    let label = host.label();
    let holder = holder_clause(pid);
    match pid {
        OrphanPid::Known(_) => format!(
            "the {label} port {} is held by a process this MCP host did not start{holder} \
             — an orphan — and it was signalled but did not go. This host owns no child of \
             its own.",
            *port
        ),
        OrphanPid::Unknown => format!(
            "the {label} port {} is held by a process this MCP host did not start — an \
             orphan — and no process could be named for it, so it could not be stopped. \
             This host owns no child of its own.",
            *port
        ),
    }
}
