//! Route `tools/call` to the right handler.

use cobalt_mcp_protocol::message::{McpRequest, McpResponse};
use serde_json::{Value, json};

use super::{
    commands::{parse_detail, parse_filter, render_catalogue},
    run::{parse_run, render_outcome},
};
use crate::{
    hosts::{HostName, HostPair, HostSet, McpHostSpec},
    lifecycle::{HostLifecycle, InstanceId, RecordedInstance},
    mcp::{content::tool_error, control, tools::ToolName},
};

const HOST_ARG: &str = "host";

const INSTANCE_ARG: &str = "instance";

// The `host` argument as a name, or none when the call left it out.
fn named_host(args: &Value) -> Result<Option<HostName>, String> {
    match args.get(HOST_ARG) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(word)) => Ok(Some(HostName::new(word.clone()))),
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
    host: &McpHostSpec,
    args: &Value,
    lifecycle: &dyn HostLifecycle,
) -> InstanceChoice {
    let recorded = lifecycle.instances();
    match args.get(INSTANCE_ARG) {
        None | Some(Value::Null) => {
            if host.runs_many_instances() && !recorded.is_empty() {
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

fn instance_refusal(host: &McpHostSpec, recorded: &[RecordedInstance]) -> ToolCallOutcome {
    let named: Vec<&str> = recorded
        .iter()
        .map(|instance| instance.id().as_str())
        .collect();
    let held = if named.is_empty() {
        format!(
            "this MCP host records no {} instance. Call {} to start one",
            host.name(),
            host.launch_tool_name(),
        )
    } else {
        format!(
            "this MCP host records these {} instances: {}",
            host.name(),
            named.join(", "),
        )
    };
    ToolCallOutcome::Result(tool_error(&format!(
        "`{INSTANCE_ARG}` must name a recorded instance, and {held}. A `run`, `commands`, \
         `stop` or `logs` call on {} names the instance it acts on, and the launch reply \
         carries the id.",
        host.name()
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
    let requested = match named_host(args) {
        Ok(requested) => requested,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    let (host, pair) = match hosts.resolve(requested.as_ref()) {
        Ok(resolved) => resolved,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    match tool {
        ToolName::Launch => {
            let (link, lifecycle) = pair.parts();
            control::handle_launch(&host, args, link, lifecycle)
        }
        ToolName::Stop => control::handle_stop(&host, args, pair.lifecycle()),
        ToolName::Logs => control::handle_logs(&host, args, pair.lifecycle()),
        ToolName::Commands => match aim_at_instance(&host, args, pair) {
            Err(refusal) => refusal,
            Ok(_) => handle_commands(args, pair),
        },
        ToolName::Run => match aim_at_instance(&host, args, pair) {
            Err(refusal) => refusal,
            Ok(instance) => handle_run(args, pair, instance.as_ref()),
        },
    }
}

// Point a forwarded call at the instance it named; a multi-instance host naming none is refused.
fn aim_at_instance(
    host: &McpHostSpec,
    args: &Value,
    pair: &mut HostPair<'_>,
) -> Result<Option<InstanceId>, ToolCallOutcome> {
    let recorded = pair.lifecycle().instances();
    let choice = resolve_instance(host, args, pair.lifecycle());
    match choice {
        InstanceChoice::Refused(refusal) => Err(refusal),
        InstanceChoice::Unnamed if host.runs_many_instances() => {
            Err(instance_refusal(host, &recorded))
        }
        InstanceChoice::Unnamed => Ok(None),
        InstanceChoice::Named(instance) => {
            if let Some(held) = recorded.iter().find(|held| *held.id() == instance) {
                pair.link().retarget(held.port());
            }
            Ok(Some(instance))
        }
    }
}

fn handle_commands(args: &Value, pair: &mut HostPair<'_>) -> ToolCallOutcome {
    let detail = match parse_detail(args) {
        Ok(detail) => detail,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    let filter = match parse_filter(args) {
        Ok(filter) => filter,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    match pair.link().request(McpRequest::Catalogue) {
        Ok(McpResponse::Catalogue(catalogue)) => {
            ToolCallOutcome::Result(render_catalogue(&catalogue, detail, filter.as_ref()))
        }
        Ok(other) => ToolCallOutcome::Result(tool_error(&format!(
            "the host answered a Catalogue request with {other:?}"
        ))),
        Err(err) => ToolCallOutcome::Result(tool_error(&err.to_string())),
    }
}

fn handle_run(
    args: &Value,
    pair: &mut HostPair<'_>,
    instance: Option<&InstanceId>,
) -> ToolCallOutcome {
    let run = match parse_run(args) {
        Ok(run) => run,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    let child_dir = match instance {
        Some(named) => pair.lifecycle().instance_working_dir(named),
        None => pair.lifecycle().child_working_dir(),
    };
    match pair.link().request(McpRequest::Run(run)) {
        Ok(McpResponse::Outcome(outcome)) => {
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

    use super::{HostName, named_host};

    #[test]
    fn a_call_is_aimed_by_its_host_argument() {
        assert_eq!(named_host(&json!({})), Ok(None));
        assert_eq!(
            named_host(&json!({ "host": "alpha" })),
            Ok(Some(HostName::new("alpha".to_owned())))
        );
        assert_eq!(
            named_host(&json!({ "host": "beta" })),
            Ok(Some(HostName::new("beta".to_owned())))
        );
    }

    #[test]
    fn a_host_argument_that_is_not_a_string_is_rejected() {
        assert!(named_host(&json!({ "host": 7 })).is_err());
    }
}
