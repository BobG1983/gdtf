use serde_json::{Value, json};

use crate::{
    hosts::QaHost,
    lifecycle::{
        BootTimeout, FailureTail, LaunchFailure, LaunchOutcome, LaunchSpec, OrphanPid, OutputTail,
        StopOutcome, TailLines,
    },
    link::QaPort,
    mcp::content::{text_content, tool_error},
};

pub(super) fn render_launch(
    host: QaHost,
    outcome: &LaunchOutcome,
    requested: &LaunchSpec,
) -> Value {
    match outcome {
        LaunchOutcome::Launched {
            port,
            pid,
            instance,
        } => text_content(&json!({
            "status": "launched", "port": **port, "pid": **pid,
            "instance": instance.as_str(),
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

fn resolved_working_dir(spec: &LaunchSpec) -> String {
    spec.resolved_working_dir().map_or_else(
        || "<unknown>".to_owned(),
        |dir| dir.to_string_lossy().into_owned(),
    )
}

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
        LaunchFailure::NoFreePort { requested: port } => format!(
            "this MCP host's own {label} records hold port {}, and no port just above it was \
             free, so nothing was started. Call {} for one of the recorded instances to free \
             a port, then try again.",
            **port,
            host.stop_tool_name(),
        ),
        LaunchFailure::Timeout { tail, waited } => timeout_message(host, *waited, requested, tail),
        LaunchFailure::ExitedEarly(tail) => format!(
            "the {label} exited before it became ready. output tail (stdout and stderr, \
             interleaved):\n{}",
            tail.as_str()
        ),
    }
}

fn timeout_message(
    host: QaHost,
    waited: BootTimeout,
    requested: &LaunchSpec,
    tail: &FailureTail,
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
         directory and launch again; the output tail below shows how far the build got. \
         output tail (stdout and stderr, interleaved):\n{}",
        *waited,
        requested.package().as_str(),
        resolved_working_dir(requested),
        requested.package().as_str(),
        tail.as_str()
    )
}

fn holder_clause(pid: OrphanPid) -> String {
    match pid {
        OrphanPid::Known(child) => format!(" (process {})", *child),
        OrphanPid::Unknown => String::new(),
    }
}

fn holder_field(pid: OrphanPid) -> Value {
    match pid {
        OrphanPid::Known(child) => json!(*child),
        OrphanPid::Unknown => Value::Null,
    }
}

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

pub(super) fn render_logs(host: QaHost, max: TailLines, tail: Option<&OutputTail>) -> Value {
    let Some(tail) = tail else {
        return text_content(&json!({
            "status": "not_running",
            "host": host.label(),
        }));
    };
    let lines: Vec<&str> = if tail.is_empty() {
        Vec::new()
    } else {
        tail.lines().collect()
    };
    text_content(&json!({
        "status": "running",
        "host": host.label(),
        "max_lines": *max,
        "lines": lines,
    }))
}

#[cfg(test)]
mod test {
    use super::{LaunchFailure, LaunchOutcome, QaHost, QaPort, render_launch};

    #[test]
    fn a_full_port_search_is_not_reported_as_an_orphan() {
        let outcome = LaunchOutcome::Failed(LaunchFailure::NoFreePort {
            requested: QaPort::new(7617),
        });

        let reply = render_launch(QaHost::Editor, &outcome, &QaHost::Editor.default_spec());

        let Some(text) = reply["content"][0]["text"].as_str() else {
            unreachable!("a launch failure renders as text content: {reply}");
        };
        assert_eq!(reply["isError"], serde_json::json!(true), "{reply}");
        assert!(
            !text.contains("orphan"),
            "this host's own records hold the port, so the reply must not send the author after \
             an orphan: {text}"
        );
    }
}
