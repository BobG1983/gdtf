//! The non-capture reply arms: a host error, the receipts, the views, and the editor's two
//! query replies (GTW-741 onward). The capture arms live in
//! [`screenshot`](super::screenshot).

use gdtf_qa_protocol::{
    envelope::{FocusControlReceipt, InjectReceipt, QaError, QaResponse, StepperReceipt},
    view::{AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet, RequestKindNet},
};
use serde_json::json;

use crate::mcp::{call::render::render_response, content::tool_error, tools::ToolName};

/// A game-side error renders as an MCP tool error, not a normal payload.
#[test]
fn render_response_maps_game_error_to_tool_error() {
    let rendered = render_response(
        ToolName::QueryState,
        &QaResponse::Error(QaError::NoBattle),
        None,
    );
    assert_eq!(rendered["isError"], json!(true));
}

/// A `start_battle` reply is an app-flow snapshot and must render as normal text
/// content. Without its arm it fell into the catch-all and a SUCCESSFUL navigation
/// reported "the game returned a response that does not match the request" (GTW-760).
#[test]
fn render_response_start_battle_renders_the_app_flow_snapshot() {
    let view = AppFlowView::new(
        AppStateNet::Running,
        BattleActiveNet::new(true),
        vec![RequestKindNet::GetBattleState],
        CaughtUpNet::new(true),
        None,
        None,
    );
    let rendered = render_response(ToolName::StartBattle, &QaResponse::AppFlow(view), None);
    assert_eq!(rendered["isError"], json!(false));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("an app-flow reply carries text content");
    };
    assert!(text.contains("Running"));
}

/// An injected-intent receipt renders as non-error text content.
#[test]
fn render_response_injected_is_text_content() {
    let rendered = render_response(
        ToolName::SendInput,
        &QaResponse::Injected(InjectReceipt::Queued),
        None,
    );
    assert_eq!(rendered["isError"], json!(false));
    assert_eq!(rendered["content"][0]["type"], json!("text"));
}

/// A menu-item activation receipt renders as non-error text content. Without its arm it
/// would fall into the catch-all and a SUCCESSFUL click would report "the game returned a
/// response that does not match the request" (the GTW-760 half-split class, GTW-787).
#[test]
fn render_response_menu_item_activated_is_text_content() {
    let rendered = render_response(
        ToolName::ActivateMenuItem,
        &QaResponse::MenuItemActivated(
            gdtf_qa_protocol::envelope::MenuActivationReceipt::Activated,
        ),
        None,
    );
    assert_eq!(rendered["isError"], json!(false));
    assert_eq!(rendered["content"][0]["type"], json!("text"));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a menu-item activation reply carries text content");
    };
    assert!(text.contains("Activated"), "rendered: {text}");
}

/// A stepper-control receipt renders as non-error text content. Without its arm it would
/// fall into the catch-all and a SUCCESSFUL drive would report "the game returned a
/// response that does not match the request" (the GTW-760 half-split class, GTW-766).
#[test]
fn render_response_stepper_controlled_is_text_content() {
    let rendered = render_response(
        ToolName::StepperControl,
        &QaResponse::StepperControlled(StepperReceipt::Latched),
        None,
    );
    assert_eq!(rendered["isError"], json!(false));
    assert_eq!(rendered["content"][0]["type"], json!("text"));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a stepper-control reply carries text content");
    };
    assert!(text.contains("Latched"), "rendered: {text}");
}

/// A focus-control receipt renders as non-error text content. Without its arm it would
/// fall into the catch-all and a SUCCESSFUL focus drive would report "the game returned a
/// response that does not match the request" (the GTW-760 half-split class, GTW-802).
#[test]
fn render_response_focus_controlled_is_text_content() {
    let rendered = render_response(
        ToolName::FocusControl,
        &QaResponse::FocusControlled(FocusControlReceipt::Applied),
        None,
    );
    assert_eq!(rendered["isError"], json!(false));
    assert_eq!(rendered["content"][0]["type"], json!("text"));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a focus-control reply carries text content");
    };
    assert!(text.contains("Applied"), "rendered: {text}");
}

/// The editor's two query replies render as non-error text content. Without their
/// arms a SUCCESSFUL editor query would fall into the catch-all and report "the host
/// returned a response that does not match the request" — the GTW-760 half-split class
/// (GTW-808).
#[test]
fn render_response_editor_query_replies_are_text_content() {
    use gdtf_qa_protocol::view::{
        EditorQueryKind, EditorQueryOptionsView, EditorQueryReply, EditorQueryTopicView,
        EditorQueryView, EditorReadinessNet,
    };

    let options = render_response(
        ToolName::GetEditorQueryOptions,
        &QaResponse::EditorQueryOptions(EditorQueryOptionsView::new(
            EditorReadinessNet::Load,
            vec![EditorQueryTopicView::offered(EditorQueryKind::Validation)],
        )),
        None,
    );
    assert_eq!(options["isError"], json!(false));
    let Some(text) = options["content"][0]["text"].as_str() else {
        unreachable!("an editor options reply carries text content");
    };
    assert!(text.contains("Validation"), "rendered: {text}");
    assert!(text.contains("Load"), "rendered: {text}");

    let query = render_response(
        ToolName::QueryEditor,
        &QaResponse::EditorQuery(EditorQueryReply::new(
            EditorReadinessNet::Editing,
            EditorQueryView::Readiness(EditorReadinessNet::Editing),
        )),
        None,
    );
    assert_eq!(query["isError"], json!(false));
    let Some(text) = query["content"][0]["text"].as_str() else {
        unreachable!("an editor query reply carries text content");
    };
    assert!(text.contains("Editing"), "rendered: {text}");
}

/// The tool-error helper marks the block as an error.
#[test]
fn tool_error_sets_is_error() {
    assert_eq!(tool_error("boom")["isError"], json!(true));
}
