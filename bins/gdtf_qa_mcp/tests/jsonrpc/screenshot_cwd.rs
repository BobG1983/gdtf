//! A capture written by a child running in a DIFFERENT directory than the host comes back
//! as an image, over the real JSON-RPC surface — for BOTH children (GTW-923).
//!
//! These go through `dispatch` → `handle_tool_call` → `render_response`, so they cover the
//! whole read path, not the resolution helper on its own. The setup is the defect's exact
//! shape: the child reports the RELATIVE path its own default produces
//! (`target/qa_screenshots/…` for the game, `target/editor_qa_screenshots/…` for the
//! editor), the PNG exists only under a temp directory that is NOT the test process's
//! current directory, and the lifecycle reports that temp directory as the child's. Before
//! the fix the host read the reported path against its own directory, found nothing, and
//! answered with "could not be read" text instead of the image.
//!
//! The two children carry DIFFERENT bytes, so a reply proves which child's file was read.

use std::{
    fs,
    path::{Path, PathBuf},
    process,
    time::SystemTime,
};

use gdtf_qa_mcp::{
    ChildPid, HostLifecycle, HostPair, HostSet, LaunchOutcome, LaunchSpec, McpError, QaLink,
    QaPort, StopOutcome, WorkingDir, base64::encode_standard, dispatch,
};
use gdtf_qa_protocol::envelope::{
    QaError, QaRequest, QaResponse, ScreenshotAfterResult, ScreenshotPathNet, ScreenshotResult,
};
use serde_json::{Value, json};

/// The relative path the GAME's child reports, mirroring `QaShotDir::default()` in
/// `crates/gdtf_app/src/dev/net_qa/screenshot/path.rs`.
const GAME_RELATIVE_SHOT: &str = "target/qa_screenshots/gtw923_game.png";

/// The relative path the EDITOR's child reports, mirroring `EditorQaShotDir::default()` in
/// `crates/gdtf_content_editor/src/net_qa/screenshot/path.rs`.
const EDITOR_RELATIVE_SHOT: &str = "target/editor_qa_screenshots/gtw923_editor.png";

/// The bytes standing in for the GAME's PNG.
const GAME_SHOT_BYTES: &[u8] = b"gtw923-game-png-bytes";

/// The bytes standing in for the EDITOR's PNG.
const EDITOR_SHOT_BYTES: &[u8] = b"gtw923-editor-png-bytes";

/// A link that answers both capture requests with one saved RELATIVE path — what a real
/// child reports, since neither default is absolute.
struct RelativeShotLink(&'static str);

impl QaLink for RelativeShotLink {
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        let path = ScreenshotPathNet::new(self.0.to_owned());
        match request {
            QaRequest::TakeScreenshot { .. } => {
                Ok(QaResponse::Screenshot(ScreenshotResult::Saved(path)))
            }
            QaRequest::ScreenshotAfter { .. } => Ok(QaResponse::ScreenshotAfter(
                ScreenshotAfterResult::Saved(path),
            )),
            _ => Ok(QaResponse::Error(QaError::BadRequest)),
        }
    }
}

/// A lifecycle reporting a running child launched in the directory it carries — the one
/// fact the render path needs to open a child-written relative path.
struct ChildInDirLifecycle(PathBuf);

impl HostLifecycle for ChildInDirLifecycle {
    fn launch(&mut self, port: QaPort, _spec: &LaunchSpec) -> LaunchOutcome {
        LaunchOutcome::Launched {
            port,
            pid: ChildPid::new(process::id()),
        }
    }

    fn stop(&mut self, _port: QaPort) -> StopOutcome {
        StopOutcome::NotRunning
    }

    fn stop_owned(&mut self) -> StopOutcome {
        StopOutcome::NotRunning
    }

    fn child_working_dir(&self) -> Option<WorkingDir> {
        Some(WorkingDir::new(self.0.clone()))
    }
}

/// A fresh temp directory holding both children's captures at their own relative defaults,
/// asserted to be a different directory than the test process's own.
fn a_child_tree_holding_both_captures() -> PathBuf {
    let nanos = SystemTime::UNIX_EPOCH
        .elapsed()
        .map_or(0, |since| since.as_nanos());
    let root = std::env::temp_dir().join(format!("gtw923-{}-{nanos}", process::id()));
    for (relative, bytes) in [
        (GAME_RELATIVE_SHOT, GAME_SHOT_BYTES),
        (EDITOR_RELATIVE_SHOT, EDITOR_SHOT_BYTES),
    ] {
        let file = root.join(relative);
        let Some(parent) = file.parent() else {
            unreachable!("the capture path has a parent directory");
        };
        let Ok(()) = fs::create_dir_all(parent) else {
            unreachable!("the test can create the child's capture directory");
        };
        let Ok(()) = fs::write(&file, bytes) else {
            unreachable!("the test can write the child's capture");
        };
    }
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };
    assert_ne!(root, here, "the child's tree differs from the host's");
    assert!(
        !here.join(GAME_RELATIVE_SHOT).exists() && !here.join(EDITOR_RELATIVE_SHOT).exists(),
        "neither capture may exist under the HOST's directory, or a host-relative read \
         would pass for the wrong reason",
    );
    root
}

/// Dispatch `line` with both children reporting relative capture paths and both lifecycles
/// reporting `child_dir` as the running child's directory.
fn dispatch_with_child_in(line: &str, child_dir: &Path) -> Value {
    let (mut game_link, mut editor_link) = (
        RelativeShotLink(GAME_RELATIVE_SHOT),
        RelativeShotLink(EDITOR_RELATIVE_SHOT),
    );
    let (mut game_life, mut editor_life) = (
        ChildInDirLifecycle(child_dir.to_path_buf()),
        ChildInDirLifecycle(child_dir.to_path_buf()),
    );
    let mut hosts = HostSet::new(
        HostPair::new(&mut game_link, &mut game_life),
        HostPair::new(&mut editor_link, &mut editor_life),
    );
    let Some(response) = dispatch(line, &mut hosts) else {
        unreachable!("a request with an id yields a response line");
    };
    serde_json::from_str(&response).unwrap_or(Value::Null)
}

/// Assert `response` carries exactly `bytes` as PNG image content.
fn assert_image_of(response: &Value, bytes: &[u8]) {
    assert_eq!(
        response["result"]["isError"],
        json!(false),
        "the capture must render as an image, not an error: {response}",
    );
    assert_eq!(response["result"]["content"][0]["type"], json!("image"));
    assert_eq!(
        response["result"]["content"][0]["mimeType"],
        json!("image/png")
    );
    assert_eq!(
        response["result"]["content"][0]["data"],
        json!(encode_standard(bytes)),
        "the image must be the bytes the CHILD's file holds: {response}",
    );
}

/// `take_screenshot` against the GAME: the child ran elsewhere, and its capture still comes
/// back as the image.
#[test]
fn a_game_capture_from_another_directory_relays_as_an_image() {
    let root = a_child_tree_holding_both_captures();
    let response = dispatch_with_child_in(
        r#"{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"take_screenshot","arguments":{}}}"#,
        &root,
    );
    assert_image_of(&response, GAME_SHOT_BYTES);
    drop(fs::remove_dir_all(&root));
}

/// `take_screenshot` aimed at the EDITOR: the same property, on the host GTW-875's
/// `working_dir` was added for. A host-side resolution covers both children with one rule,
/// which is why the editor needs no separate fix (GTW-923 clause 5).
#[test]
fn an_editor_capture_from_another_directory_relays_as_an_image() {
    let root = a_child_tree_holding_both_captures();
    let response = dispatch_with_child_in(
        r#"{"jsonrpc":"2.0","id":41,"method":"tools/call","params":{"name":"take_screenshot","arguments":{"host":"editor"}}}"#,
        &root,
    );
    assert_image_of(&response, EDITOR_SHOT_BYTES);
    drop(fs::remove_dir_all(&root));
}

/// `screenshot_after`'s capture reads the same way — the second read site, which had its own
/// copy of the host-relative read (GTW-923 clause 4).
#[test]
fn a_screenshot_after_capture_from_another_directory_relays_as_an_image() {
    let root = a_child_tree_holding_both_captures();
    let response = dispatch_with_child_in(
        r#"{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{"name":"screenshot_after","arguments":{"intent":"EndTurn","frame_delay":2}}}"#,
        &root,
    );
    assert_image_of(&response, GAME_SHOT_BYTES);
    drop(fs::remove_dir_all(&root));
}

/// A capture that is genuinely absent from the child's directory is STILL a tool error
/// naming both paths — the reported one and the one the host opened. Removing a false
/// failure must not introduce a false success (GTW-923 clause 6).
#[test]
fn a_missing_capture_is_still_a_tool_error_naming_both_paths() {
    let root = a_child_tree_holding_both_captures();
    let file = root.join(GAME_RELATIVE_SHOT);
    let Ok(()) = fs::remove_file(&file) else {
        unreachable!("the test can delete the child's capture");
    };

    let response = dispatch_with_child_in(
        r#"{"jsonrpc":"2.0","id":43,"method":"tools/call","params":{"name":"take_screenshot","arguments":{}}}"#,
        &root,
    );
    assert_eq!(
        response["result"]["isError"],
        json!(true),
        "a missing file must not render as an image: {response}",
    );
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("a tool error carries text content");
    };
    assert!(text.contains(GAME_RELATIVE_SHOT), "rendered: {text}");
    let Some(shown) = file.to_str() else {
        unreachable!("the temp path is valid UTF-8");
    };
    assert!(
        text.contains(shown),
        "the error names the path the host actually opened: {text}",
    );
    assert!(
        response["result"]["content"]
            .as_array()
            .is_some_and(|blocks| blocks.iter().all(|block| block["type"] != json!("image"))),
        "an unreadable capture must produce no image content: {response}",
    );
    drop(fs::remove_dir_all(&root));
}
