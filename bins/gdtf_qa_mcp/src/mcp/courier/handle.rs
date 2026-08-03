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

const HOST_ARG: &str = "host";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallOutcome {
        Result(Value),
        Invalid(String),
}

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
        ToolName::Stop => control::handle_stop(host, pair.lifecycle()),
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
