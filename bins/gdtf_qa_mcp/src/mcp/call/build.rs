//! [`build_request`] — a tool's `arguments` → the [`QaRequest`] it maps onto.

use gdtf_qa_protocol::{
    envelope::QaRequest,
    ids::{EventCap, FrameDelay, ShotName},
    intent::NetIntent,
};
use serde_json::Value;

use crate::mcp::tools::ToolName;

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
        ToolName::ScreenshotAfter => Ok(QaRequest::ScreenshotAfter {
            intent:      parse_intent(args)?,
            frame_delay: parse_frame_delay(args)?,
            name:        parse_name(args)?,
        }),
        ToolName::AppFlow => Ok(QaRequest::GetAppFlow),
        // The lifecycle tools are handled before this point (in `handle_tool_call`), so
        // they never map onto a wire request; reaching here would be a routing bug.
        ToolName::LaunchGame | ToolName::StopGame => {
            Err("launch_game / stop_game are host-local tools, not game requests".to_owned())
        }
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

/// Parse the optional `take_screenshot` / `screenshot_after` `name` file stem.
fn parse_name(args: &Value) -> Result<Option<ShotName>, String> {
    match args.get("name") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(stem)) => Ok(Some(ShotName::new(stem.clone()))),
        Some(_) => Err("`name` must be a string".to_owned()),
    }
}

/// Parse the required `screenshot_after` `frame_delay` argument.
fn parse_frame_delay(args: &Value) -> Result<FrameDelay, String> {
    let Some(value) = args.get("frame_delay") else {
        return Err("`screenshot_after` needs a `frame_delay` argument".to_owned());
    };
    let Some(raw) = value.as_u64() else {
        return Err("`frame_delay` must be a non-negative integer".to_owned());
    };
    let Ok(narrow) = u32::try_from(raw) else {
        return Err("`frame_delay` is too large".to_owned());
    };
    Ok(FrameDelay::new(narrow))
}

#[cfg(test)]
mod tests {
    use gdtf_qa_protocol::{envelope::QaRequest, ids::FrameDelay, intent::NetIntent};
    use serde_json::json;

    use super::build_request;
    use crate::mcp::tools::ToolName;

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

    /// `screenshot_after` builds a `QaRequest::ScreenshotAfter` from its `intent`,
    /// `frame_delay`, and optional `name`.
    #[test]
    fn build_request_parses_screenshot_after() {
        let args = json!({ "intent": "EndTurn", "frame_delay": 3, "name": "post_turn" });
        let request = build_request(ToolName::ScreenshotAfter, &args);
        assert_eq!(
            request,
            Ok(QaRequest::ScreenshotAfter {
                intent:      NetIntent::EndTurn,
                frame_delay: FrameDelay::new(3),
                name:        Some(gdtf_qa_protocol::ids::ShotName::new("post_turn".to_owned())),
            })
        );
    }

    /// `screenshot_after` rejects a missing `frame_delay`.
    #[test]
    fn build_request_screenshot_after_requires_frame_delay() {
        let args = json!({ "intent": "EndTurn" });
        assert!(build_request(ToolName::ScreenshotAfter, &args).is_err());
    }
}
