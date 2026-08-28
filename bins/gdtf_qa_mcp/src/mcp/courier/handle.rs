//! Route `tools/call` to the right handler.

use gdtf_qa_protocol::message::{QaRequest, QaResponse};
use serde_json::{Value, json};

use super::{
    commands::{parse_detail, parse_filter, render_catalogue},
    run::{parse_run, render_outcome},
};
use crate::{
    hosts::{HostSet, QaHost},
    lifecycle::{HostLifecycle, InstanceId, RecordedInstance},
    mcp::{content::tool_error, control, tools::ToolName},
};

const HOST_ARG: &str = "host";

const INSTANCE_ARG: &str = "instance";

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

/// Outcome of a tool call: JSON result or invalid-params message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallOutcome {
    /// Successful tool result payload.
    Result(Value),
    /// Argument validation failure.
    Invalid(String),
}

/// Which instance a call acts on, or the reply that refuses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceChoice {
    /// The call named a recorded instance.
    Named(InstanceId),
    /// The call named none, and this host needs none.
    Unnamed,
    /// The call cannot be answered, and this is what it gets back.
    Refused(ToolCallOutcome),
}

/// Resolve a call's `instance` argument against what its host records.
#[must_use]
pub fn resolve_instance(
    host: QaHost,
    args: &Value,
    lifecycle: &dyn HostLifecycle,
) -> InstanceChoice {
    let recorded = lifecycle.instances();
    match args.get(INSTANCE_ARG) {
        None | Some(Value::Null) => {
            if matches!(host, QaHost::Editor) && !recorded.is_empty() {
                return InstanceChoice::Refused(instance_refusal(host, &recorded));
            }
            InstanceChoice::Unnamed
        }
        Some(Value::String(word)) => recorded
            .iter()
            .find(|instance| instance.id().as_str() == word)
            .map_or_else(
                || InstanceChoice::Refused(instance_refusal(host, &recorded)),
                |instance| InstanceChoice::Named(instance.id().clone()),
            ),
        Some(other) => InstanceChoice::Refused(ToolCallOutcome::Invalid(format!(
            "`{INSTANCE_ARG}` must be a string, not {other}"
        ))),
    }
}

fn instance_refusal(host: QaHost, recorded: &[RecordedInstance]) -> ToolCallOutcome {
    let named: Vec<&str> = recorded
        .iter()
        .map(|instance| instance.id().as_str())
        .collect();
    let list = if named.is_empty() {
        "none".to_owned()
    } else {
        named.join(", ")
    };
    ToolCallOutcome::Result(tool_error(&format!(
        "`{INSTANCE_ARG}` must name one of the {} instances this MCP host records: {list}. An \
         editor `stop` or `logs` names the instance it acts on, and the launch reply carries \
         the id.",
        host.label(),
    )))
}

/// Dispatch one MCP `tools/call` request.
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
        ToolName::Launch => {
            let (link, lifecycle) = pair.parts();
            control::handle_launch(host, args, link, lifecycle)
        }
        ToolName::Stop => control::handle_stop(host, args, pair.lifecycle()),
        ToolName::Logs => control::handle_logs(host, args, pair.lifecycle()),
        ToolName::Commands => handle_commands(args, pair),
        ToolName::Run => handle_run(args, pair),
    }
}

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

    #[test]
    fn a_call_is_aimed_by_its_host_argument() {
        assert_eq!(resolve_host(&json!({})), Ok(QaHost::Game));
        assert_eq!(resolve_host(&json!({ "host": "game" })), Ok(QaHost::Game));
        assert_eq!(
            resolve_host(&json!({ "host": "editor" })),
            Ok(QaHost::Editor),
        );
    }

    #[test]
    fn an_unknown_host_word_is_rejected() {
        assert!(resolve_host(&json!({ "host": "edtior" })).is_err());
        assert!(resolve_host(&json!({ "host": 7 })).is_err());
    }
}
