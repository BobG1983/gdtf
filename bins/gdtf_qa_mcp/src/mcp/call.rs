//! The `tools/call` handler — tool name + arguments → a game request → MCP content
//! (GTW-741).
//!
//! One place owns the tool ⇄ protocol mapping: [`build_request`] turns a call's
//! arguments into a [`QaRequest`], the [`GameLink`] carries it to the game, and
//! [`render_response`] turns the [`QaResponse`] back into an MCP content block. A link
//! failure or a game-side [`QaError`] becomes an MCP tool error (`isError: true`) — never
//! a crash, and never a fabricated screenshot.

use gdtf_qa_protocol::{
    envelope::{QaError, QaRequest, QaResponse, ScreenshotResult},
    ids::{EventCap, ShotName},
    intent::NetIntent,
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{base64::encode_standard, game::GameLink, mcp::tools::ToolName};

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

/// Handle a `tools/call` request: resolve the tool, build the game request, carry it over
/// the link, and render the reply.
///
/// A missing `params`, a missing / unknown tool name, or an un-buildable request is an
/// [`Invalid`](ToolCallOutcome::Invalid) (JSON-RPC invalid-params). A link failure is a
/// tool error inside a normal result.
#[must_use]
pub fn handle_tool_call(params: Option<&Value>, game: &mut dyn GameLink) -> ToolCallOutcome {
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
    match build_request(tool, args) {
        Ok(request) => match game.request(request) {
            Ok(response) => ToolCallOutcome::Result(render_response(tool, &response)),
            Err(err) => ToolCallOutcome::Result(tool_error(&err.to_string())),
        },
        Err(message) => ToolCallOutcome::Invalid(message),
    }
}

/// Build the [`QaRequest`] a tool call maps onto from its `arguments`.
///
/// # Errors
///
/// A human-readable message when an argument is missing or the wrong shape.
pub fn build_request(tool: ToolName, args: &Value) -> Result<QaRequest, String> {
    match tool {
        ToolName::SendInput => Ok(QaRequest::Inject(parse_intent(args)?)),
        ToolName::QueryState => Ok(QaRequest::GetBattleState),
        ToolName::GetOutput => Ok(QaRequest::GetOutput {
            max: parse_max(args)?,
        }),
        ToolName::TakeScreenshot => Ok(QaRequest::TakeScreenshot {
            name: parse_name(args)?,
        }),
        ToolName::AppFlow => Ok(QaRequest::GetAppFlow),
    }
}

/// Parse the `send_input` `intent` argument — a JSON object OR a compact-RON string.
fn parse_intent(args: &Value) -> Result<NetIntent, String> {
    let Some(value) = args.get("intent") else {
        return Err("`send_input` needs an `intent` argument".to_owned());
    };
    match value {
        Value::String(text) => ron::from_str::<NetIntent>(text)
            .map_err(|err| format!("could not parse RON intent: {err}")),
        other => serde_json::from_value::<NetIntent>(other.clone())
            .map_err(|err| format!("could not parse JSON intent: {err}")),
    }
}

/// Parse the optional `get_output` `max` cap.
fn parse_max(args: &Value) -> Result<Option<EventCap>, String> {
    match args.get("max") {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let Some(raw) = value.as_u64() else {
                return Err("`max` must be a non-negative integer".to_owned());
            };
            let Ok(narrow) = u32::try_from(raw) else {
                return Err("`max` is too large".to_owned());
            };
            Ok(Some(EventCap::new(narrow)))
        }
    }
}

/// Parse the optional `take_screenshot` `name` file stem.
fn parse_name(args: &Value) -> Result<Option<ShotName>, String> {
    match args.get("name") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(stem)) => Ok(Some(ShotName::new(stem.clone()))),
        Some(_) => Err("`name` must be a string".to_owned()),
    }
}

/// Turn a game reply into the MCP content block for `tool`.
///
/// A game-side [`QaError`] on any tool becomes a tool error; a screenshot reply routes
/// through `render_screenshot`; every other tool renders its payload as pretty JSON
/// text. A reply variant that does not match the tool is itself a tool error.
#[must_use]
pub fn render_response(tool: ToolName, response: &QaResponse) -> Value {
    match (tool, response) {
        (_, QaResponse::Error(err)) => tool_error(&qa_error_message(*err)),
        (ToolName::SendInput, QaResponse::Injected(receipt)) => text_content(receipt),
        (ToolName::QueryState, QaResponse::Battle(view)) => text_content(view),
        (ToolName::GetOutput, QaResponse::Output(batch)) => text_content(batch),
        (ToolName::AppFlow, QaResponse::AppFlow(view)) => text_content(view),
        (ToolName::TakeScreenshot, QaResponse::Screenshot(result)) => render_screenshot(result),
        _ => tool_error("the game returned a response that does not match the request"),
    }
}

/// Render a screenshot reply: read the saved PNG and return it as image content, or a tool
/// error if the capture timed out or the file cannot be read (never a fabricated image).
fn render_screenshot(result: &ScreenshotResult) -> Value {
    match result {
        ScreenshotResult::Saved(path) => match std::fs::read(path.as_str()) {
            Ok(bytes) => image_content(&encode_standard(&bytes)),
            Err(err) => tool_error(&format!(
                "screenshot saved to {} but could not be read: {err}",
                path.as_str()
            )),
        },
        ScreenshotResult::TimedOut => {
            tool_error("screenshot capture timed out before it landed on disk")
        }
    }
}

/// A pretty-JSON text content block (the successful, non-image tool payload).
fn text_content<T: Serialize>(value: &T) -> Value {
    let text =
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "<unserializable>".to_owned());
    json!({ "content": [ { "type": "text", "text": text } ], "isError": false })
}

/// A base64 PNG image content block.
fn image_content(data: &str) -> Value {
    json!({
        "content": [ { "type": "image", "data": data, "mimeType": "image/png" } ],
        "isError": false
    })
}

/// A tool-error content block (`isError: true`) carrying a human-readable reason.
fn tool_error(message: &str) -> Value {
    json!({ "content": [ { "type": "text", "text": message } ], "isError": true })
}

/// A human-readable message for a game-side protocol error.
fn qa_error_message(error: QaError) -> String {
    format!("the game rejected the request: {error:?}")
}

#[cfg(test)]
mod tests {
    use gdtf_qa_protocol::{
        envelope::{
            InjectReceipt, QaError, QaRequest, QaResponse, ScreenshotPathNet, ScreenshotResult,
        },
        intent::NetIntent,
    };
    use serde_json::json;

    use super::{ToolName, build_request, render_response, tool_error};

    /// `send_input` accepts a compact-RON intent string.
    #[test]
    fn build_request_parses_ron_intent() {
        let args = json!({ "intent": "Reload" });
        let request = build_request(ToolName::SendInput, &args);
        assert_eq!(request, Ok(QaRequest::Inject(NetIntent::Reload)));
    }

    /// `send_input` also accepts a JSON-object intent.
    #[test]
    fn build_request_parses_json_intent() {
        let args = json!({ "intent": { "SetStance": { "stance": "Standing" } } });
        let request = build_request(ToolName::SendInput, &args);
        let Ok(QaRequest::Inject(NetIntent::SetStance { .. })) = request else {
            unreachable!("a SetStance JSON object parses to an Inject(SetStance): {request:?}");
        };
    }

    /// `get_output` maps its optional `max` onto the request.
    #[test]
    fn build_request_reads_output_cap() {
        let capped = build_request(ToolName::GetOutput, &json!({ "max": 3 }));
        let Ok(QaRequest::GetOutput { max: Some(cap) }) = capped else {
            unreachable!("a numeric max yields a capped GetOutput: {capped:?}");
        };
        assert_eq!(*cap, 3);
        let uncapped = build_request(ToolName::GetOutput, &json!({}));
        assert_eq!(uncapped, Ok(QaRequest::GetOutput { max: None }));
    }

    /// A game-side error renders as an MCP tool error, not a normal payload.
    #[test]
    fn render_response_maps_game_error_to_tool_error() {
        let rendered = render_response(ToolName::QueryState, &QaResponse::Error(QaError::NoBattle));
        assert_eq!(rendered["isError"], json!(true));
    }

    /// A timed-out screenshot renders as a tool error, never a fabricated image.
    #[test]
    fn render_response_screenshot_timeout_is_tool_error() {
        let rendered = render_response(
            ToolName::TakeScreenshot,
            &QaResponse::Screenshot(ScreenshotResult::TimedOut),
        );
        assert_eq!(rendered["isError"], json!(true));
        let Some(content) = rendered["content"][0]["text"].as_str() else {
            unreachable!("a tool error carries a text reason");
        };
        assert!(content.contains("timed out"));
    }

    /// An injected-intent receipt renders as non-error text content.
    #[test]
    fn render_response_injected_is_text_content() {
        let rendered = render_response(
            ToolName::SendInput,
            &QaResponse::Injected(InjectReceipt::Queued),
        );
        assert_eq!(rendered["isError"], json!(false));
        assert_eq!(rendered["content"][0]["type"], json!("text"));
    }

    /// A saved screenshot with a readable file renders as base64 image content.
    #[test]
    fn render_response_screenshot_saved_is_image_content() {
        let path = std::env::temp_dir().join(format!("gdtf_qa_mcp_{}.png", std::process::id()));
        let bytes: &[u8] = b"fake-png-bytes";
        if std::fs::write(&path, bytes).is_err() {
            unreachable!("the test can write its temp screenshot");
        }
        let Some(path_str) = path.to_str() else {
            unreachable!("the temp path is valid UTF-8");
        };
        let rendered = render_response(
            ToolName::TakeScreenshot,
            &QaResponse::Screenshot(ScreenshotResult::Saved(ScreenshotPathNet::new(
                path_str.to_owned(),
            ))),
        );
        drop(std::fs::remove_file(&path));
        assert_eq!(rendered["isError"], json!(false));
        assert_eq!(rendered["content"][0]["type"], json!("image"));
        assert_eq!(rendered["content"][0]["mimeType"], json!("image/png"));
        assert_eq!(
            rendered["content"][0]["data"],
            json!(super::encode_standard(bytes))
        );
    }

    /// The tool-error helper marks the block as an error.
    #[test]
    fn tool_error_sets_is_error() {
        assert_eq!(tool_error("boom")["isError"], json!(true));
    }
}
