//! The reply → MCP content-block mapping (GTW-741, GTW-749, GTW-808).

use gdtf_qa_protocol::envelope::{QaError, QaResponse, ScreenshotAfterResult, ScreenshotResult};
use serde_json::Value;

use crate::{
    base64::encode_standard,
    mcp::{
        content::{image_content, text_content, tool_error},
        tools::ToolName,
    },
};

/// Turn a host's reply into the MCP content block for `tool`.
///
/// A host-side [`QaError`] on any tool becomes a tool error; a screenshot reply routes
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
        (ToolName::GetEditorQueryOptions, QaResponse::EditorQueryOptions(view)) => {
            text_content(view)
        }
        (ToolName::QueryEditor, QaResponse::EditorQuery(reply)) => text_content(reply),
        _ => tool_error("the host returned a response that does not match the request"),
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

/// A human-readable message for a host-side protocol error.
fn qa_error_message(error: QaError) -> String {
    format!("the host rejected the request: {error:?}")
}
