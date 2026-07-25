//! [`render_response`] — a game reply → the MCP content block for the tool that asked.

use gdtf_qa_protocol::envelope::{QaError, QaResponse, ScreenshotAfterResult, ScreenshotResult};
use serde_json::Value;

use crate::{
    base64::encode_standard,
    mcp::{
        content::{image_content, text_content, tool_error},
        tools::ToolName,
    },
};

/// Turn a game reply into the MCP content block for `tool`.
///
/// A game-side [`QaError`] on any tool becomes a tool error; a screenshot reply routes
/// through `render_screenshot`, a `screenshot_after` reply through
/// `render_screenshot_after`; every other tool renders its payload as pretty JSON text. A
/// reply variant that does not match the tool is itself a tool error.
#[must_use]
pub fn render_response(tool: ToolName, response: &QaResponse) -> Value {
    match (tool, response) {
        (_, QaResponse::Error(err)) => tool_error(&qa_error_message(*err)),
        (ToolName::SendInput, QaResponse::Injected(receipt)) => text_content(receipt),
        (ToolName::QueryState, QaResponse::Battle(view)) => text_content(view),
        (ToolName::GetOutput, QaResponse::Output(batch)) => text_content(batch),
        // `start_battle` is answered with an app-flow snapshot, not a bespoke reply: the
        // useful thing to hand back is the state the navigation reached, which is exactly
        // what `app_flow` reports (`net_qa::start_battle::drive_start_battle`).
        (ToolName::AppFlow | ToolName::StartBattle, QaResponse::AppFlow(view)) => {
            text_content(view)
        }
        (ToolName::TakeScreenshot, QaResponse::Screenshot(result)) => render_screenshot(result),
        (ToolName::ScreenshotAfter, QaResponse::ScreenshotAfter(result)) => {
            render_screenshot_after(result)
        }
        (ToolName::StepperControl, QaResponse::StepperControlled(receipt)) => text_content(receipt),
        (ToolName::ActivateMenuItem, QaResponse::MenuItemActivated(receipt)) => {
            text_content(receipt)
        }
        (ToolName::FocusControl, QaResponse::FocusControlled(receipt)) => text_content(receipt),
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

/// Render a `screenshot_after` reply: a rejected embedded intent is a tool error naming
/// the reason (NO image — no capture was ever attempted); an accepted intent's capture
/// renders exactly like `render_screenshot` (read the saved PNG, or a tool error on a
/// timeout / unreadable file — never a fabricated image).
fn render_screenshot_after(result: &ScreenshotAfterResult) -> Value {
    match result {
        ScreenshotAfterResult::Rejected(reason) => {
            tool_error(&format!("the embedded intent was rejected: {reason:?}"))
        }
        ScreenshotAfterResult::Saved(path) => match std::fs::read(path.as_str()) {
            Ok(bytes) => image_content(&encode_standard(&bytes)),
            Err(err) => tool_error(&format!(
                "screenshot saved to {} but could not be read: {err}",
                path.as_str()
            )),
        },
        ScreenshotAfterResult::TimedOut => {
            tool_error("screenshot capture timed out before it landed on disk")
        }
    }
}

/// A human-readable message for a game-side protocol error.
fn qa_error_message(error: QaError) -> String {
    format!("the game rejected the request: {error:?}")
}

#[cfg(test)]
mod tests {
    use gdtf_qa_protocol::{
        envelope::{
            FocusControlReceipt, InjectReceipt, QaError, QaResponse, RejectReason,
            ScreenshotAfterResult, ScreenshotPathNet, ScreenshotResult, StepperReceipt,
        },
        view::{AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet, RequestKindNet},
    };
    use serde_json::json;

    use super::render_response;
    use crate::mcp::{content::tool_error, tools::ToolName};

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
            json!(super::encode_standard(bytes))
        );
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
        let path =
            std::env::temp_dir().join(format!("gdtf_qa_mcp_after_{}.png", std::process::id()));
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
            json!(super::encode_standard(bytes))
        );
    }
}
