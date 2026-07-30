//! The reply → content-block mapping's unit tests (GTW-741 onward).

use gdtf_qa_protocol::{
    envelope::{
        CaptureAimNet, FocusControlReceipt, InjectReceipt, QaError, QaResponse, RejectReason,
        ScreenshotAfterResult, ScreenshotPathNet, ScreenshotResult, StepperReceipt,
    },
    view::{AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet, RequestKindNet},
};
use serde_json::json;

use super::response::render_response;
use crate::{
    base64::encode_standard,
    mcp::{content::tool_error, tools::ToolName},
};

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

/// A refused capture renders as a tool error carrying the host's detail, and NO image block.
///
/// This is the surface an agent actually reads. If this arm ever handed back a normal content
/// block — or dropped the detail — a capture that was never taken would read as a screenshot,
/// which is the GTW-922 failure (a blank frame answering as a real one) moved one layer out.
#[test]
fn render_response_screenshot_target_not_rendered_is_tool_error() {
    let rendered = render_response(
        ToolName::TakeScreenshot,
        &QaResponse::Screenshot(ScreenshotResult::TargetNotRendered(CaptureAimNet::new(
            "the camera renders into Window(PrimaryWindow)".to_owned(),
        ))),
    );
    assert_eq!(rendered["isError"], json!(true));
    assert_eq!(rendered["content"][0]["type"], json!("text"));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a tool error carries a text reason");
    };
    assert!(text.contains("no screenshot was taken"), "rendered: {text}");
    assert!(
        text.contains("the camera renders into Window(PrimaryWindow)"),
        "the host's detail must reach the client: {text}"
    );
    assert!(
        rendered["content"]
            .as_array()
            .is_some_and(|blocks| blocks.iter().all(|block| block["type"] != json!("image"))),
        "a refused capture must produce no image content: {rendered}"
    );
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
    let rendered = render_response(ToolName::StartBattle, &QaResponse::AppFlow(view));
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
    );
    assert_eq!(rendered["isError"], json!(false));
    assert_eq!(rendered["content"][0]["type"], json!("text"));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a focus-control reply carries text content");
    };
    assert!(text.contains("Applied"), "rendered: {text}");
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
        json!(encode_standard(bytes))
    );
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

/// A rejected embedded intent renders as a tool error naming the reason, never an
/// image.
#[test]
fn render_response_screenshot_after_rejected_is_tool_error() {
    let rendered = render_response(
        ToolName::ScreenshotAfter,
        &QaResponse::ScreenshotAfter(ScreenshotAfterResult::Rejected(RejectReason::NotOffered)),
    );
    assert_eq!(rendered["isError"], json!(true));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a tool error carries a text reason");
    };
    assert!(text.contains("NotOffered"), "rendered: {text}");
}

/// An accepted `screenshot_after` whose capture saved renders as base64 image content,
/// exactly like `take_screenshot`.
#[test]
fn render_response_screenshot_after_saved_is_image_content() {
    let path = std::env::temp_dir().join(format!("gdtf_qa_mcp_after_{}.png", std::process::id()));
    let bytes: &[u8] = b"fake-png-bytes";
    if std::fs::write(&path, bytes).is_err() {
        unreachable!("the test can write its temp screenshot");
    }
    let Some(path_str) = path.to_str() else {
        unreachable!("the temp path is valid UTF-8");
    };
    let rendered = render_response(
        ToolName::ScreenshotAfter,
        &QaResponse::ScreenshotAfter(ScreenshotAfterResult::Saved(ScreenshotPathNet::new(
            path_str.to_owned(),
        ))),
    );
    drop(std::fs::remove_file(&path));
    assert_eq!(rendered["isError"], json!(false));
    assert_eq!(rendered["content"][0]["type"], json!("image"));
    assert_eq!(
        rendered["content"][0]["data"],
        json!(encode_standard(bytes))
    );
}
