//! The command-carrying argument parsers — the DEV procgen stepper drive (GTW-766) and the
//! focus drive (GTW-802). Each accepts a JSON value OR a compact-RON string, the same dual
//! shape the intent parser accepts.

use gdtf_qa_protocol::envelope::{FocusCommandNet, StepperCommandNet};
use serde_json::Value;

/// Parse the required `stepper_control` `command` argument.
pub(super) fn parse_stepper_command(args: &Value) -> Result<StepperCommandNet, String> {
    let Some(value) = args.get("command") else {
        return Err("`stepper_control` needs a `command` argument".to_owned());
    };
    match value {
        Value::String(text) => ron::from_str::<StepperCommandNet>(text)
            .map_err(|err| format!("could not parse RON stepper command: {err}")),
        other => serde_json::from_value::<StepperCommandNet>(other.clone())
            .map_err(|err| format!("could not parse JSON stepper command: {err}")),
    }
}

/// Parse the required `focus_control` `command` argument — e.g. `{"ActivateTarget": 42}`,
/// `"Activate"`, `{"Step": "Next"}`, or `{"Focus": 42}`.
pub(super) fn parse_focus_command(args: &Value) -> Result<FocusCommandNet, String> {
    let Some(value) = args.get("command") else {
        return Err("`focus_control` needs a `command` argument".to_owned());
    };
    match value {
        Value::String(text) => ron::from_str::<FocusCommandNet>(text)
            .map_err(|err| format!("could not parse RON focus command: {err}")),
        other => serde_json::from_value::<FocusCommandNet>(other.clone())
            .map_err(|err| format!("could not parse JSON focus command: {err}")),
    }
}
