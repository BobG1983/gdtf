//! The two host-local entry points — [`handle_launch`] and [`handle_stop`] — and the
//! `port` argument they read.

use serde_json::Value;

use super::render::{render_launch, render_stop};
use crate::{
    hosts::QaHost,
    lifecycle::{HostLifecycle, LaunchOutcome},
    link::{QaLink, QaPort},
    mcp::{call::ToolCallOutcome, launch_args::parse_launch_spec},
};

/// Handle a `launch_game` / `launch_editor` call: read the launch recipe, ensure `host`'s
/// child is running on the chosen port, re-point that host's link at it, and render the
/// outcome.
///
/// `host` decides only the DEFAULTS — the port, the package, the features, and the two QA
/// variables the child reads. Everything else is the call's own arguments, so a caller can
/// point either tool at another package or another checkout (GTW-808).
#[must_use]
pub fn handle_launch(
    host: QaHost,
    args: &Value,
    link: &mut dyn QaLink,
    lifecycle: &mut dyn HostLifecycle,
) -> ToolCallOutcome {
    let port = match parse_port(host, args) {
        Ok(port) => port,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    let spec = match parse_launch_spec(&host.default_spec(), args) {
        Ok(spec) => spec,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    let outcome = lifecycle.launch(port, &spec);
    // Point the forwarding link at whichever port the child is actually on, so subsequent
    // tool calls for this host reach the child this launch ensured.
    match &outcome {
        LaunchOutcome::Launched { port, .. } | LaunchOutcome::AlreadyRunning { port, .. } => {
            link.retarget(*port);
        }
        LaunchOutcome::Failed(_) => {}
    }
    ToolCallOutcome::Result(render_launch(host, &outcome, &spec))
}

/// Handle a `stop_game` / `stop_editor` call: stop that host's running child (if any) and
/// render the outcome. The two hosts' managers are separate, so stopping one never
/// touches the other.
#[must_use]
pub fn handle_stop(lifecycle: &mut dyn HostLifecycle) -> ToolCallOutcome {
    ToolCallOutcome::Result(render_stop(&lifecycle.stop()))
}

/// Parse the optional `port` argument, defaulting to the port `host` reads from its own
/// environment variable.
pub(super) fn parse_port(host: QaHost, args: &Value) -> Result<QaPort, String> {
    match args.get("port") {
        None | Some(Value::Null) => Ok(host.port_from_env()),
        Some(value) => {
            let Some(raw) = value.as_u64() else {
                return Err("`port` must be a non-negative integer".to_owned());
            };
            let Ok(narrow) = u16::try_from(raw) else {
                return Err("`port` must be in the range 0..=65535".to_owned());
            };
            Ok(QaPort::new(narrow))
        }
    }
}
