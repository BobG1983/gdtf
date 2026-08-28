//! Control tool handlers (launch, stop, logs).

use serde_json::Value;

use super::render::{render_launch, render_logs, render_stop};
use crate::{
    hosts::QaHost,
    lifecycle::{HostLifecycle, LaunchOutcome, TailLines},
    link::{QaLink, QaPort},
    mcp::{InstanceChoice, ToolCallOutcome, launch_args::parse_launch_spec, resolve_instance},
};

/// Launch (or attach to) a host with optional recipe overrides.
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
    match &outcome {
        LaunchOutcome::Launched { port, .. } | LaunchOutcome::AlreadyRunning { port, .. } => {
            link.retarget(*port);
        }
        LaunchOutcome::Failed(_) => {}
    }
    ToolCallOutcome::Result(render_launch(host, &outcome, &spec))
}

/// Stop a host's named instance, or its child (or an orphan on its port).
#[must_use]
pub fn handle_stop(
    host: QaHost,
    args: &Value,
    lifecycle: &mut dyn HostLifecycle,
) -> ToolCallOutcome {
    let choice = resolve_instance(host, args, lifecycle);
    let outcome = match choice {
        InstanceChoice::Refused(refusal) => return refusal,
        InstanceChoice::Named(instance) => lifecycle.stop_instance(&instance),
        InstanceChoice::Unnamed => lifecycle.stop(host.port_from_env()),
    };
    ToolCallOutcome::Result(render_stop(host, &outcome))
}

/// Return recent output for a host's named instance, or for its child.
#[must_use]
pub fn handle_logs(
    host: QaHost,
    args: &Value,
    lifecycle: &mut dyn HostLifecycle,
) -> ToolCallOutcome {
    let max = match parse_max_lines(args) {
        Ok(max) => max,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    let choice = resolve_instance(host, args, lifecycle);
    let tail = match choice {
        InstanceChoice::Refused(refusal) => return refusal,
        InstanceChoice::Named(instance) => lifecycle.instance_output(&instance, max),
        InstanceChoice::Unnamed => lifecycle.child_output(max),
    };
    ToolCallOutcome::Result(render_logs(host, max, tail.as_ref()))
}

fn parse_max_lines(args: &Value) -> Result<TailLines, String> {
    match args.get("max_lines") {
        None | Some(Value::Null) => Ok(TailLines::default()),
        Some(value) => {
            let Some(raw) = value.as_u64() else {
                return Err("`max_lines` must be a non-negative integer".to_owned());
            };
            let Ok(narrow) = usize::try_from(raw) else {
                return Err("`max_lines` is larger than this platform can address".to_owned());
            };
            Ok(TailLines::new(narrow))
        }
    }
}

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
