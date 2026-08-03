use serde_json::Value;

use super::render::{render_launch, render_logs, render_stop};
use crate::{
    hosts::QaHost,
    lifecycle::{HostLifecycle, LaunchOutcome, TailLines},
    link::{QaLink, QaPort},
    mcp::{ToolCallOutcome, launch_args::parse_launch_spec},
};

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

#[must_use]
pub fn handle_stop(host: QaHost, lifecycle: &mut dyn HostLifecycle) -> ToolCallOutcome {
    let outcome = lifecycle.stop(host.port_from_env());
    ToolCallOutcome::Result(render_stop(host, &outcome))
}

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
    ToolCallOutcome::Result(render_logs(host, max, lifecycle.child_output(max).as_ref()))
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
