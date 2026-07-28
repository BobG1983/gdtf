//! The scalar argument parsers — the event cap, the file stem, the frame delay, the seed,
//! the situation name, the focus token, and the editor query topic.

use gdtf_qa_protocol::{
    ids::{EventCap, FocusTargetNet, FrameDelay, SeedNet, ShotName, SituationRef},
    view::EditorQueryKind,
};
use serde_json::Value;

use crate::mcp::editor_topic::{topic_from_wire, topic_wire_names};

/// Parse the optional `get_output` `max` cap.
pub(super) fn parse_max(args: &Value) -> Result<Option<EventCap>, String> {
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
pub(super) fn parse_name(args: &Value) -> Result<Option<ShotName>, String> {
    match args.get("name") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(stem)) => Ok(Some(ShotName::new(stem.clone()))),
        Some(_) => Err("`name` must be a string".to_owned()),
    }
}

/// Parse the required `start_battle` `situation` name.
///
/// The name is passed through as the client wrote it: which situations exist is the
/// game's to know, and it answers an unknown one with a typed rejection. Validating a
/// shipped-situation list here too would be a second copy of that truth, free to drift.
pub(super) fn parse_situation(args: &Value) -> Result<SituationRef, String> {
    match args.get("situation") {
        None | Some(Value::Null) => Err("`start_battle` needs a `situation` argument".to_owned()),
        Some(Value::String(name)) => Ok(SituationRef::new(name.clone())),
        Some(_) => Err("`situation` must be a string".to_owned()),
    }
}

/// Parse the optional `start_battle` `seed`, which pins the procgen RNG.
pub(super) fn parse_seed(args: &Value) -> Result<Option<SeedNet>, String> {
    match args.get("seed") {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let Some(raw) = value.as_u64() else {
                return Err("`seed` must be a non-negative integer".to_owned());
            };
            Ok(Some(SeedNet::new(raw)))
        }
    }
}

/// Parse the required `activate_menu_item` `token` argument — the numeric focus token a
/// menu enumeration handed out.
pub(super) fn parse_token(args: &Value) -> Result<FocusTargetNet, String> {
    let Some(value) = args.get("token") else {
        return Err("`activate_menu_item` needs a `token` argument".to_owned());
    };
    let Some(bits) = value.as_u64() else {
        return Err("`token` must be a non-negative integer".to_owned());
    };
    Ok(FocusTargetNet::new(bits))
}

/// Parse the required `screenshot_after` `frame_delay` argument.
pub(super) fn parse_frame_delay(args: &Value) -> Result<FrameDelay, String> {
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

/// Read the required `topic` argument of a `query_editor` call.
///
/// # Errors
///
/// A message listing every legal topic when the argument is missing, is not a string, or
/// names no topic — never a silent fallback to a default topic, which would answer a
/// question the caller did not ask (GTW-808).
pub(super) fn parse_topic(args: &Value) -> Result<EditorQueryKind, String> {
    let Some(name) = args.get("topic").and_then(Value::as_str) else {
        return Err(format!(
            "`topic` is required and must be one of: {}",
            topic_wire_names().join(", ")
        ));
    };
    topic_from_wire(name).ok_or_else(|| {
        format!(
            "unknown editor topic `{name}`; expected one of: {}",
            topic_wire_names().join(", ")
        )
    })
}
