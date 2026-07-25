//! Argument-parsing pins for the tool → request map.

use gdtf_qa_protocol::{
    envelope::{FocusCommandNet, FocusStepNet, QaRequest, StepperCommandNet},
    ids::{FocusTargetNet, FrameDelay, SeedNet, SituationRef},
    intent::NetIntent,
};
use serde_json::json;

use crate::mcp::{call::build::build_request, tools::ToolName};

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

/// `send_input` forwards a GTW-783 raw-input keypress intent — the MCP host exposes the
/// keyboard-drive capability through the same generic injector tool, no per-intent tool.
#[test]
fn build_request_parses_keypress_intent() {
    let args = json!({ "intent": { "PressKey": { "key": { "Key": "Tab" } } } });
    let request = build_request(ToolName::SendInput, &args);
    let Ok(QaRequest::Inject(NetIntent::PressKey { .. })) = request else {
        unreachable!("a PressKey JSON object parses to an Inject(PressKey): {request:?}");
    };
}

/// `send_input` forwards a raw-input hover intent (a window pixel position).
#[test]
fn build_request_parses_hover_intent() {
    let args = json!({ "intent": { "Hover": { "at": { "x": 120, "y": 48 } } } });
    let request = build_request(ToolName::SendInput, &args);
    let Ok(QaRequest::Inject(NetIntent::Hover { .. })) = request else {
        unreachable!("a Hover JSON object parses to an Inject(Hover): {request:?}");
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

/// `start_battle` builds a `QaRequest::StartBattle` carrying the situation and the
/// optional seed — the client half of the navigation path an agent needs to reach a
/// battle at all (GTW-760).
#[test]
fn build_request_parses_start_battle_with_seed() {
    let args = json!({ "situation": "skirmish", "seed": 42 });
    let request = build_request(ToolName::StartBattle, &args);
    assert_eq!(
        request,
        Ok(QaRequest::StartBattle {
            situation: SituationRef::new("skirmish".to_owned()),
            seed:      Some(SeedNet::new(42)),
        })
    );
}

/// An omitted `seed` leaves the game to choose one.
#[test]
fn build_request_start_battle_seed_is_optional() {
    let args = json!({ "situation": "skirmish" });
    let request = build_request(ToolName::StartBattle, &args);
    assert_eq!(
        request,
        Ok(QaRequest::StartBattle {
            situation: SituationRef::new("skirmish".to_owned()),
            seed:      None,
        })
    );
}

/// `start_battle` rejects a missing or non-string `situation` rather than inventing
/// a default — which situation to start is never guessed on the client's behalf.
#[test]
fn build_request_start_battle_requires_a_situation() {
    assert!(build_request(ToolName::StartBattle, &json!({})).is_err());
    assert!(build_request(ToolName::StartBattle, &json!({ "situation": 7 })).is_err());
}

/// An unknown situation name is passed through to the game, not rejected here — the
/// game owns which situations exist and answers with a typed rejection, so this
/// builder never holds a second copy of that list.
#[test]
fn build_request_start_battle_passes_an_unknown_situation_through() {
    let args = json!({ "situation": "no-such-situation" });
    let request = build_request(ToolName::StartBattle, &args);
    assert_eq!(
        request,
        Ok(QaRequest::StartBattle {
            situation: SituationRef::new("no-such-situation".to_owned()),
            seed:      None,
        })
    );
}

/// `stepper_control` accepts a compact-RON command string (`Next` / `Skip`).
#[test]
fn build_request_parses_ron_stepper_command() {
    let next = build_request(ToolName::StepperControl, &json!({ "command": "Next" }));
    assert_eq!(next, Ok(QaRequest::StepperControl(StepperCommandNet::Next)));
    let skip = build_request(ToolName::StepperControl, &json!({ "command": "Skip" }));
    assert_eq!(skip, Ok(QaRequest::StepperControl(StepperCommandNet::Skip)));
}

/// `stepper_control` also accepts a JSON-object command — the `Auto` on/off carrier.
#[test]
fn build_request_parses_json_stepper_auto_command() {
    let args = json!({ "command": { "Auto": { "running": true } } });
    let request = build_request(ToolName::StepperControl, &args);
    let Ok(QaRequest::StepperControl(StepperCommandNet::Auto { running })) = request else {
        unreachable!("an Auto JSON object parses to a StepperControl(Auto): {request:?}");
    };
    assert!(*running, "the Auto command carries its on/off state");
}

/// `stepper_control` rejects a missing `command` rather than inventing a default.
#[test]
fn build_request_stepper_control_requires_a_command() {
    assert!(build_request(ToolName::StepperControl, &json!({})).is_err());
}

/// `activate_menu_item` builds a `QaRequest::ActivateMenuItem` from its numeric `token`
/// — the client half of the menu-click path (GTW-787).
#[test]
fn build_request_parses_activate_menu_item_token() {
    let args = json!({ "token": 4_294_967_297_u64 });
    let request = build_request(ToolName::ActivateMenuItem, &args);
    assert_eq!(
        request,
        Ok(QaRequest::ActivateMenuItem(
            gdtf_qa_protocol::ids::FocusTargetNet::new(4_294_967_297)
        ))
    );
}

/// `activate_menu_item` rejects a missing or non-integer `token` rather than inventing
/// one.
#[test]
fn build_request_activate_menu_item_requires_a_token() {
    assert!(build_request(ToolName::ActivateMenuItem, &json!({})).is_err());
    assert!(build_request(ToolName::ActivateMenuItem, &json!({ "token": "x" })).is_err());
}

/// `focus_control` accepts the compact-RON bare `Activate` command string (GTW-802).
#[test]
fn build_request_parses_ron_focus_activate() {
    let request = build_request(ToolName::FocusControl, &json!({ "command": "Activate" }));
    assert_eq!(
        request,
        Ok(QaRequest::FocusControl(FocusCommandNet::Activate))
    );
}

/// `focus_control` accepts the JSON-object commands: an activate-by-token (the "click this
/// control" shape an agent reads straight off `app_flow`'s focusables) and a focus step.
#[test]
fn build_request_parses_json_focus_commands() {
    let activate = build_request(
        ToolName::FocusControl,
        &json!({ "command": { "ActivateTarget": 4_294_967_297_u64 } }),
    );
    assert_eq!(
        activate,
        Ok(QaRequest::FocusControl(FocusCommandNet::ActivateTarget(
            FocusTargetNet::new(4_294_967_297)
        )))
    );
    let step = build_request(
        ToolName::FocusControl,
        &json!({ "command": { "Step": "Next" } }),
    );
    assert_eq!(
        step,
        Ok(QaRequest::FocusControl(FocusCommandNet::Step(
            FocusStepNet::Next
        )))
    );
}

/// `focus_control` rejects a missing or unparseable `command` rather than inventing one.
#[test]
fn build_request_focus_control_requires_a_command() {
    assert!(build_request(ToolName::FocusControl, &json!({})).is_err());
    assert!(build_request(ToolName::FocusControl, &json!({ "command": "Nope" })).is_err());
}
