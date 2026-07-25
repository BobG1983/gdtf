//! The `NetIntent` argument parser — a JSON object OR a compact-RON string.

use gdtf_qa_protocol::intent::NetIntent;
use serde_json::Value;

/// Parse the `send_input` / `screenshot_after` `intent` argument — a JSON object OR a
/// compact-RON string.
pub(super) fn parse_intent(args: &Value) -> Result<NetIntent, String> {
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
