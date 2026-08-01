//! [`handle_tool_call`] — resolve the tool, pick the host it acts on, then either drive
//! that host's lifecycle (the four launch / stop tools) or build the request, carry it
//! over that host's link, and render the reply.

use serde_json::{Value, json};

use super::{build::build_request, render::render_response};
use crate::{
    hosts::{HostSet, QaHost},
    mcp::{content::tool_error, control, tools::ToolName},
};

/// The tool argument that aims a two-host tool at one of the children.
const HOST_ARG: &str = "host";

/// Which host this call acts on: the `host` argument when the tool accepts one and the call
/// names it, else the tool's own [`ToolName::host`].
///
/// A `host` value that names neither child is rejected as invalid params rather than silently
/// falling back to the default, so a typo reaches the caller instead of screenshotting the
/// wrong process (GTW-880).
fn resolve_host(tool: ToolName, args: &Value) -> Result<QaHost, String> {
    if !tool.accepts_host_argument() {
        return Ok(tool.host());
    }
    match args.get(HOST_ARG) {
        None | Some(Value::Null) => Ok(tool.host()),
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

/// Handle a `tools/call` request: resolve the tool, take the link + lifecycle belonging to
/// the host it names, then either drive that host's lifecycle (the four launch / stop
/// tools) or build the request, carry it over that host's link, and render the reply.
///
/// The host comes from [`ToolName::host`], so an editor tool never travels over the game's
/// link and the two children are driven independently — which is what lets a game and an
/// editor be up at the same time (GTW-808). A tool that
/// [`accepts_host_argument`](ToolName::accepts_host_argument) — today only `take_screenshot`,
/// since both children can capture a frame (GTW-880) — may be aimed at either child by a
/// `host` argument on the call, and a `host` value naming neither child is rejected as
/// invalid params rather than quietly defaulting.
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
    let host = match resolve_host(tool, args) {
        Ok(host) => host,
        Err(message) => return ToolCallOutcome::Invalid(message),
    };
    let pair = hosts.pair(host);
    if tool.is_launch() {
        // A launch starts that host's process rather than forwarding a request to it.
        let (link, lifecycle) = pair.parts();
        return control::handle_launch(host, args, link, lifecycle);
    }
    if tool.is_stop() {
        return control::handle_stop(host, pair.lifecycle());
    }
    // Every other tool maps onto a `QaRequest` carried over its host's link. The child's
    // own directory is read BEFORE the request travels, because the render step needs it to
    // open a capture the child wrote at a relative path (GTW-923) — the host's current
    // directory is not the child's whenever a launch named a `working_dir` of its own.
    let child_dir = pair.lifecycle().child_working_dir();
    match build_request(tool, args) {
        Ok(request) => match pair.link().request(request) {
            Ok(response) => {
                ToolCallOutcome::Result(render_response(tool, &response, args, child_dir.as_ref()))
            }
            Err(err) => ToolCallOutcome::Result(tool_error(&err.to_string())),
        },
        Err(message) => ToolCallOutcome::Invalid(message),
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::{QaHost, ToolName, resolve_host};

    /// `take_screenshot` is aimed by its `host` argument, and defaults to the game when the
    /// call names none — the routing GTW-880 adds so one tool can capture either child.
    #[test]
    fn take_screenshot_is_aimed_by_its_host_argument() {
        assert_eq!(
            resolve_host(ToolName::TakeScreenshot, &json!({})),
            Ok(QaHost::Game),
            "a call naming no host must reach the game",
        );
        assert_eq!(
            resolve_host(ToolName::TakeScreenshot, &json!({ "host": "editor" })),
            Ok(QaHost::Editor),
            "host=editor must reach the EDITOR's link, not the game's",
        );
        assert_eq!(
            resolve_host(ToolName::TakeScreenshot, &json!({ "host": "game" })),
            Ok(QaHost::Game),
        );
    }

    /// A `host` value naming neither child is rejected rather than quietly falling back —
    /// a typo must not screenshot the wrong process.
    #[test]
    fn an_unknown_host_word_is_rejected() {
        assert!(
            resolve_host(ToolName::TakeScreenshot, &json!({ "host": "edtior" })).is_err(),
            "a misspelled host must be invalid params, not a silent default",
        );
        assert!(
            resolve_host(ToolName::TakeScreenshot, &json!({ "host": 7 })).is_err(),
            "a non-string host must be invalid params",
        );
    }

    /// Every other tool keeps the host it names, whatever a call asks for: an `inject` has
    /// no meaning in the editor and a `query_editor` none in the game.
    #[test]
    fn a_single_host_tool_ignores_a_host_argument() {
        let aimed_at_editor = json!({ "host": "editor" });
        assert_eq!(
            resolve_host(ToolName::SendInput, &aimed_at_editor),
            Ok(QaHost::Game),
        );
        let aimed_at_game = json!({ "host": "game" });
        assert_eq!(
            resolve_host(ToolName::QueryEditor, &aimed_at_game),
            Ok(QaHost::Editor),
        );
    }
}
