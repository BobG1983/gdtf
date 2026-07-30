//! The two capture arms: WHICH file a saved capture is read from (GTW-923), and every
//! refusal that must stay a tool error rather than an image (GTW-734, GTW-749, GTW-922).
//!
//! The GTW-923 tests are the ones that would fail if the host went back to reading a
//! child-written path against its own current directory: each writes the PNG under a temp
//! directory, hands `render_response` that directory as the child's, and reports a RELATIVE
//! path — the shape both children's defaults produce.

use std::{
    fs,
    path::{Path, PathBuf},
    process,
    time::SystemTime,
};

use gdtf_qa_protocol::envelope::{
    CaptureAimNet, QaResponse, RejectReason, ScreenshotAfterResult, ScreenshotPathNet,
    ScreenshotResult,
};
use serde_json::json;

use crate::{
    base64::encode_standard,
    lifecycle::WorkingDir,
    mcp::{call::render::render_response, tools::ToolName},
};

/// The bytes standing in for a captured PNG.
const SHOT_BYTES: &[u8] = b"fake-png-bytes";

/// A fresh temp directory, uniquely named, standing in for the directory a child ran in.
fn a_directory_that_is_not_the_hosts(tag: &str) -> PathBuf {
    let nanos = SystemTime::UNIX_EPOCH
        .elapsed()
        .map_or(0, |since| since.as_nanos());
    let path = std::env::temp_dir().join(format!("gtw923-{tag}-{}-{nanos}", process::id()));
    let Ok(()) = fs::create_dir_all(&path) else {
        unreachable!("the test can create its temp directory");
    };
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };
    assert_ne!(path, here, "the child's directory differs from the host's");
    path
}

/// Write [`SHOT_BYTES`] at `relative` under `root`, creating the directories it names.
fn write_capture(root: &Path, relative: &str) {
    let file = root.join(relative);
    let Some(parent) = file.parent() else {
        unreachable!("the capture path has a parent directory");
    };
    let Ok(()) = fs::create_dir_all(parent) else {
        unreachable!("the test can create the capture directory");
    };
    let Ok(()) = fs::write(&file, SHOT_BYTES) else {
        unreachable!("the test can write the capture");
    };
}

/// Assert `rendered` is PNG image content carrying [`SHOT_BYTES`].
fn assert_image_of_the_capture(rendered: &serde_json::Value) {
    assert_eq!(
        rendered["isError"],
        json!(false),
        "the capture must render as an image: {rendered}",
    );
    assert_eq!(rendered["content"][0]["type"], json!("image"));
    assert_eq!(rendered["content"][0]["mimeType"], json!("image/png"));
    assert_eq!(
        rendered["content"][0]["data"],
        json!(encode_standard(SHOT_BYTES))
    );
}

/// A `take_screenshot` capture written under the CHILD's directory, reported RELATIVE,
/// renders as the image — the host's own directory is a different one and holds no such
/// file. Reverting the resolution turns this into a "could not be read" tool error, which is
/// the whole defect (GTW-923 clauses 1, 2 and 4's first read site).
#[test]
fn a_relative_capture_is_read_under_the_childs_directory() {
    let root = a_directory_that_is_not_the_hosts("take");
    let relative = "target/qa_screenshots/gtw923.png";
    write_capture(&root, relative);

    let rendered = render_response(
        ToolName::TakeScreenshot,
        &QaResponse::Screenshot(ScreenshotResult::Saved(ScreenshotPathNet::new(
            relative.to_owned(),
        ))),
        Some(&WorkingDir::new(root.clone())),
    );

    drop(fs::remove_dir_all(&root));
    assert_image_of_the_capture(&rendered);
}

/// The same for `screenshot_after` — the SECOND read site, which carried its own copy of the
/// host-relative read (GTW-923 clause 4).
#[test]
fn a_relative_screenshot_after_capture_is_read_under_the_childs_directory() {
    let root = a_directory_that_is_not_the_hosts("after");
    let relative = "target/editor_qa_screenshots/gtw923.png";
    write_capture(&root, relative);

    let rendered = render_response(
        ToolName::ScreenshotAfter,
        &QaResponse::ScreenshotAfter(ScreenshotAfterResult::Saved(ScreenshotPathNet::new(
            relative.to_owned(),
        ))),
        Some(&WorkingDir::new(root.clone())),
    );

    drop(fs::remove_dir_all(&root));
    assert_image_of_the_capture(&rendered);
}

/// A capture the child reported as an ABSOLUTE path is read exactly there, whatever the
/// child's directory is — the resolution must not prepend a directory onto a path that
/// already names a file system-wide.
#[test]
fn an_absolute_capture_ignores_the_childs_directory() {
    let root = a_directory_that_is_not_the_hosts("absolute");
    let elsewhere = a_directory_that_is_not_the_hosts("absolute-file");
    write_capture(&elsewhere, "gtw923.png");
    let file = elsewhere.join("gtw923.png");
    let Some(shown) = file.to_str() else {
        unreachable!("the temp path is valid UTF-8");
    };

    let rendered = render_response(
        ToolName::TakeScreenshot,
        &QaResponse::Screenshot(ScreenshotResult::Saved(ScreenshotPathNet::new(
            shown.to_owned(),
        ))),
        Some(&WorkingDir::new(root.clone())),
    );

    drop(fs::remove_dir_all(&root));
    drop(fs::remove_dir_all(&elsewhere));
    assert_image_of_the_capture(&rendered);
}

/// A capture that is genuinely missing is STILL a tool error, naming both the path the child
/// reported and the path the host opened — the fix removes a false failure, it must not add
/// a false success (GTW-923 clause 6).
#[test]
fn a_missing_capture_is_a_tool_error_naming_both_paths() {
    let root = a_directory_that_is_not_the_hosts("missing");
    let relative = "target/qa_screenshots/never_written.png";

    let rendered = render_response(
        ToolName::TakeScreenshot,
        &QaResponse::Screenshot(ScreenshotResult::Saved(ScreenshotPathNet::new(
            relative.to_owned(),
        ))),
        Some(&WorkingDir::new(root.clone())),
    );

    let Some(shown) = root.join(relative).to_str().map(str::to_owned) else {
        unreachable!("the temp path is valid UTF-8");
    };
    drop(fs::remove_dir_all(&root));
    assert_eq!(rendered["isError"], json!(true));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a tool error carries a text reason");
    };
    assert!(text.contains(relative), "rendered: {text}");
    assert!(
        text.contains(&shown),
        "the error names the path the host opened: {text}",
    );
    assert!(
        rendered["content"]
            .as_array()
            .is_some_and(|blocks| blocks.iter().all(|block| block["type"] != json!("image"))),
        "an unreadable capture must produce no image content: {rendered}",
    );
}

/// With NO child directory known, a reported path is used unchanged — the host can do no
/// better, and it must still be a tool error rather than a fabricated image when that path
/// names nothing.
#[test]
fn an_unknown_child_directory_leaves_the_reported_path_alone() {
    let rendered = render_response(
        ToolName::TakeScreenshot,
        &QaResponse::Screenshot(ScreenshotResult::Saved(ScreenshotPathNet::new(
            "target/qa_screenshots/gtw923_no_child.png".to_owned(),
        ))),
        None,
    );
    assert_eq!(rendered["isError"], json!(true));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a tool error carries a text reason");
    };
    assert!(
        text.contains("target/qa_screenshots/gtw923_no_child.png"),
        "rendered: {text}",
    );
}

/// A timed-out screenshot renders as a tool error, never a fabricated image.
#[test]
fn render_response_screenshot_timeout_is_tool_error() {
    let rendered = render_response(
        ToolName::TakeScreenshot,
        &QaResponse::Screenshot(ScreenshotResult::TimedOut),
        None,
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
        None,
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

/// A rejected embedded intent renders as a tool error naming the reason, never an
/// image.
#[test]
fn render_response_screenshot_after_rejected_is_tool_error() {
    let rendered = render_response(
        ToolName::ScreenshotAfter,
        &QaResponse::ScreenshotAfter(ScreenshotAfterResult::Rejected(RejectReason::NotOffered)),
        None,
    );
    assert_eq!(rendered["isError"], json!(true));
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a tool error carries a text reason");
    };
    assert!(text.contains("NotOffered"), "rendered: {text}");
}
