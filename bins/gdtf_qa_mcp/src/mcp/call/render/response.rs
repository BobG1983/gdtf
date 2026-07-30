//! The reply → MCP content-block mapping (GTW-741, GTW-749, GTW-808, GTW-923).

use gdtf_qa_protocol::envelope::{
    QaError, QaResponse, ScreenshotAfterResult, ScreenshotPathNet, ScreenshotResult,
};
use serde_json::Value;

use super::shot_file::resolve_shot_path;
use crate::{
    base64::encode_standard,
    lifecycle::WorkingDir,
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
///
/// `child_dir` is the directory the child that answered was launched in. The two screenshot
/// arms read their PNG relative to THAT directory rather than to the MCP host's own current
/// directory (GTW-923) — the sibling `shot_file` module records why the child's directory is
/// the only one a child-written path means anything against.
#[must_use]
pub fn render_response(
    tool: ToolName,
    response: &QaResponse,
    child_dir: Option<&WorkingDir>,
) -> Value {
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
        (ToolName::TakeScreenshot, QaResponse::Screenshot(result)) => {
            render_screenshot(result, child_dir)
        }
        (ToolName::ScreenshotAfter, QaResponse::ScreenshotAfter(result)) => {
            render_screenshot_after(result, child_dir)
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

/// Read a saved capture and return it as image content, or a tool error naming why it could
/// not be read (never a fabricated image).
///
/// The ONE place either screenshot arm reads a PNG, so the resolution against the child's
/// directory cannot be fixed in one arm and forgotten in the other — before GTW-923 both
/// arms carried their own `std::fs::read(path.as_str())` against the host's current
/// directory. The error names BOTH paths, what the child reported and what the host actually
/// opened, because a directory mismatch is invisible from either one alone.
fn render_saved_shot(path: &ScreenshotPathNet, child_dir: Option<&WorkingDir>) -> Value {
    let resolved = resolve_shot_path(path, child_dir);
    match std::fs::read(&*resolved) {
        Ok(bytes) => image_content(&encode_standard(&bytes)),
        Err(err) => tool_error(&format!(
            "screenshot saved to {} (read as {}) but could not be read: {err}",
            path.as_str(),
            resolved.display(),
        )),
    }
}

/// Render a screenshot reply: read the saved PNG and return it as image content, or a tool
/// error if the capture timed out or the file cannot be read (never a fabricated image).
fn render_screenshot(result: &ScreenshotResult, child_dir: Option<&WorkingDir>) -> Value {
    match result {
        ScreenshotResult::Saved(path) => render_saved_shot(path, child_dir),
        ScreenshotResult::TimedOut => {
            tool_error("screenshot capture timed out before it landed on disk")
        }
        // NO image, deliberately: the host refused to spawn a capture it knew would read
        // pixels nothing draws into, so there is nothing to show. Handing back a blank frame
        // as a screenshot is the failure GTW-922 exists to remove.
        ScreenshotResult::TargetNotRendered(detail) => tool_error(&format!(
            "no screenshot was taken: the offscreen image the capture reads is not what the \
             app's UI camera renders into — {}",
            detail.as_str()
        )),
    }
}

/// Render a `screenshot_after` reply: a rejected embedded intent is a tool error naming
/// the reason (NO image — no capture was ever attempted); an accepted intent's capture
/// renders exactly like `render_screenshot` (read the saved PNG, or a tool error on a
/// timeout / unreadable file — never a fabricated image).
fn render_screenshot_after(
    result: &ScreenshotAfterResult,
    child_dir: Option<&WorkingDir>,
) -> Value {
    match result {
        ScreenshotAfterResult::Rejected(reason) => {
            tool_error(&format!("the embedded intent was rejected: {reason:?}"))
        }
        ScreenshotAfterResult::Saved(path) => render_saved_shot(path, child_dir),
        ScreenshotAfterResult::TimedOut => {
            tool_error("screenshot capture timed out before it landed on disk")
        }
    }
}

/// A human-readable message for a host-side protocol error.
fn qa_error_message(error: QaError) -> String {
    format!("the host rejected the request: {error:?}")
}
