//! [`handle_tool_call`] — resolve the tool, pick the host it acts on, then either drive that
//! host's lifecycle (`launch` / `stop` / `logs`) or carry a command-layer request over that
//! host's link and render the reply.

use gdtf_qa_protocol::message::{QaRequest, QaResponse};
use serde_json::{Value, json};

use super::{
    commands::{parse_detail, parse_filter, render_catalogue},
    run::{parse_run, render_outcome},
};
use crate::{
    hosts::{HostSet, QaHost},
    mcp::{content::tool_error, control, tools::ToolName},
};

/// The tool argument that aims a call at one of the children.
const HOST_ARG: &str = "host";

/// Which host this call acts on: the `host` argument when the call names one, else the game.
///
/// EVERY tool takes it, so this is one rule rather than a per-tool map. A `host` value that
/// names neither child is rejected as invalid params rather than silently falling back to
/// the default, so a typo reaches the caller instead of driving the wrong process (GTW-880).
fn resolve_host(args: &Value) -> Result<QaHost, String> {
    match args.get(HOST_ARG) {
        None | Some(Value::Null) => Ok(QaHost::Game),
        Some(Value::String(word)) => QaHost::from_label(word).ok_or_else(|| {
            format!(
                "`{HOST_ARG}` must be \"{}\" or \"{}\", not {word:?}",
                QaHost::Game.label(),
                QaHost::Editor.label(),
            )
        }),
        Some(other) => Err(format!("`{HOST_ARG}` must be a string, not {other}")),
    }
}

/// The outcome of handling a `tools/call` — either a JSON-RPC `result` object (which may
/// itself carry an MCP tool error), or an invalid-params rejection the caller renders as a
/// JSON-RPC error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallOutcome {
    /// A `tools/call` result object (an MCP content block).
    Result(Value),
    /// The call parameters were missing or invalid; the message explains why.
    Invalid(String),
}

/// Handle a `tools/call`: resolve the tool, take the link + lifecycle belonging to the host
/// the call names, then either drive that host's lifecycle or carry the request across.
///
/// A missing `params`, a missing / unknown tool name, or an un-buildable request is an
/// [`Invalid`](ToolCallOutcome::Invalid) (JSON-RPC invalid-params). A link or lifecycle
/// failure is a tool error inside a normal result.
#[must_use]
pub fn handle_tool_call(params: Option<&Value>, hosts: &mut HostSet<'_>) -> ToolCallOutcome {
    let Some(params) = params else {
        return ToolCallOutcome::Invalid("`tools/call` needs `params`".to_owned());
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return ToolCallOutcome::Invalid("`tools/call` needs a tool `name`".to_owned());
    };
    let Some(tool) = ToolName::from_wire(name) else {
        return ToolCallOutcome::Invalid(format!("unknown tool: {name}"));
    };
    let empty = json!({});
    let args = params.get("arguments").unwrap_or(&empty);
    let host = match resolve_host(args) {
        Ok(host) => host,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    let pair = hosts.pair(host);
    match tool {
        // The three host-local tools: they drive the child process itself and never reach a
        // wire.
        ToolName::Launch => {
            let (link, lifecycle) = pair.parts();
            control::handle_launch(host, args, link, lifecycle)
        }
        ToolName::Stop => control::handle_stop(host, pair.lifecycle()),
        ToolName::Logs => control::handle_logs(host, args, pair.lifecycle()),
        ToolName::Commands => handle_commands(args, pair),
        ToolName::Run => handle_run(args, pair),
    }
}

/// Handle a `commands` call: read the host's catalogue and render it at the requested detail.
fn handle_commands(args: &Value, pair: &mut crate::hosts::HostPair<'_>) -> ToolCallOutcome {
    let detail = match parse_detail(args) {
        Ok(detail) => detail,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    let filter = match parse_filter(args) {
        Ok(filter) => filter,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    match pair.link().request(QaRequest::Catalogue) {
        Ok(QaResponse::Catalogue(catalogue)) => {
            ToolCallOutcome::Result(render_catalogue(&catalogue, detail, filter.as_ref()))
        }
        Ok(other) => ToolCallOutcome::Result(tool_error(&format!(
            "the host answered a Catalogue request with {other:?}"
        ))),
        Err(err) => ToolCallOutcome::Result(tool_error(&err.to_string())),
    }
}

/// Handle a `run` call: build the request, carry it across, and render the outcome.
///
/// The child's own directory is read BEFORE the request travels, because the render step
/// needs it to open a file the child wrote at a relative path (GTW-923) — the host's current
/// directory is not the child's whenever a launch named a `working_dir` of its own.
fn handle_run(args: &Value, pair: &mut crate::hosts::HostPair<'_>) -> ToolCallOutcome {
    let run = match parse_run(args) {
        Ok(run) => run,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    let child_dir = pair.lifecycle().child_working_dir();
    match pair.link().request(QaRequest::Run(run)) {
        Ok(QaResponse::Outcome(outcome)) => {
            ToolCallOutcome::Result(render_outcome(&outcome, child_dir.as_ref()))
        }
        Ok(other) => ToolCallOutcome::Result(tool_error(&format!(
            "the host answered a Run request with {other:?}"
        ))),
        Err(err) => ToolCallOutcome::Result(tool_error(&err.to_string())),
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::{QaHost, resolve_host};

    /// A call naming no host reaches the game; naming one reaches that child.
    #[test]
    fn a_call_is_aimed_by_its_host_argument() {
        assert_eq!(resolve_host(&json!({})), Ok(QaHost::Game));
        assert_eq!(resolve_host(&json!({ "host": "game" })), Ok(QaHost::Game));
        assert_eq!(
            resolve_host(&json!({ "host": "editor" })),
            Ok(QaHost::Editor),
        );
    }

    /// A `host` value naming neither child is rejected rather than quietly falling back — a
    /// typo must not drive the wrong process.
    #[test]
    fn an_unknown_host_word_is_rejected() {
        assert!(resolve_host(&json!({ "host": "edtior" })).is_err());
        assert!(resolve_host(&json!({ "host": 7 })).is_err());
    }
}
