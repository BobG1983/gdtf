use std::{
    fs,
    path::{Path, PathBuf},
    process,
};

use gdtf_qa_mcp::{
    ChildPid, HostLifecycle, HostPair, HostSet, LaunchOutcome, LaunchSpec, McpError, OutputTail,
    QaLink, QaPort, StopOutcome, TailLines, WorkingDir, base64::encode_standard, dispatch,
};
use gdtf_qa_protocol::{
    command::{ArtifactPath, AttachmentKind, CommandOutcome, CommandReplyRon, ReplyAttachment},
    message::{QaError, QaRequest, QaResponse},
};
use serde_json::{Value, json};

const GAME_RELATIVE_SHOT: &str = "target/qa_screenshots/shotcwd_game.png";

const EDITOR_RELATIVE_SHOT: &str = "target/editor_qa_screenshots/shotcwd_editor.png";

const GAME_SHOT_BYTES: &[u8] = b"shotcwd-game-png-bytes";

const EDITOR_SHOT_BYTES: &[u8] = b"shotcwd-editor-png-bytes";

const CAPTURE_REPLY: &str = "()";

struct RelativeShotLink(&'static str);

impl QaLink for RelativeShotLink {
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        match request {
            QaRequest::Run(_) => Ok(QaResponse::Outcome(CommandOutcome::Ran {
                reply:       CommandReplyRon::new(CAPTURE_REPLY.to_owned()),
                attachments: vec![ReplyAttachment::new(
                    AttachmentKind::Png,
                    ArtifactPath::new(self.0.to_owned()),
                )],
            })),
            _ => Ok(QaResponse::Error(QaError::Malformed)),
        }
    }
}

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

    fn child_output(&self, _max: TailLines) -> Option<OutputTail> {
        None
    }
}

// The tree deletes itself when the returned handle drops, so no test shares a root.
fn a_child_tree_holding_both_captures() -> tempfile::TempDir {
    let Ok(tree) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };
    let root = tree.path();
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
    tree
}

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

fn assert_image_of(response: &Value, bytes: &[u8]) {
    assert_eq!(
        response["result"]["isError"],
        json!(false),
        "the capture must render as an image, not an error: {response}",
    );
    assert_eq!(response["result"]["content"][1]["type"], json!("image"));
    assert_eq!(
        response["result"]["content"][1]["mimeType"],
        json!("image/png")
    );
    assert_eq!(
        response["result"]["content"][1]["data"],
        json!(encode_standard(bytes)),
        "the image must be the bytes the CHILD's file holds: {response}",
    );
}

#[test]
fn a_game_capture_from_another_directory_relays_as_an_image() {
    let tree = a_child_tree_holding_both_captures();
    let response = dispatch_with_child_in(
        r#"{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"run","arguments":{"command":"capture.screenshot"}}}"#,
        tree.path(),
    );
    assert_image_of(&response, GAME_SHOT_BYTES);
}

#[test]
fn an_editor_capture_from_another_directory_relays_as_an_image() {
    let tree = a_child_tree_holding_both_captures();
    let response = dispatch_with_child_in(
        r#"{"jsonrpc":"2.0","id":41,"method":"tools/call","params":{"name":"run","arguments":{"command":"capture.screenshot","host":"editor"}}}"#,
        tree.path(),
    );
    assert_image_of(&response, EDITOR_SHOT_BYTES);
}

#[test]
fn a_missing_capture_is_still_reported_naming_both_paths() {
    let tree = a_child_tree_holding_both_captures();
    let file = tree.path().join(GAME_RELATIVE_SHOT);
    let Ok(()) = fs::remove_file(&file) else {
        unreachable!("the test can delete the child's capture");
    };

    let response = dispatch_with_child_in(
        r#"{"jsonrpc":"2.0","id":43,"method":"tools/call","params":{"name":"run","arguments":{"command":"capture.screenshot"}}}"#,
        tree.path(),
    );
    let Some(text) = response["result"]["content"][1]["text"].as_str() else {
        unreachable!("an unreadable attachment renders as a text block: {response}");
    };
    assert!(text.contains(GAME_RELATIVE_SHOT), "rendered: {text}");
    let Some(shown) = file.to_str() else {
        unreachable!("the temp path is valid UTF-8");
    };
    assert!(
        text.contains(shown),
        "the message names the path the host actually opened: {text}",
    );
    assert!(
        response["result"]["content"]
            .as_array()
            .is_some_and(|blocks| blocks.iter().all(|block| block["type"] != json!("image"))),
        "an unreadable capture must produce no image content: {response}",
    );
}

#[test]
fn two_fixture_calls_never_share_a_root() {
    let first = a_child_tree_holding_both_captures();
    let second = a_child_tree_holding_both_captures();

    assert_ne!(
        first.path(),
        second.path(),
        "two live fixture trees must be separate directories, or one test deletes another's",
    );
    for tree in [&first, &second] {
        for relative in [GAME_RELATIVE_SHOT, EDITOR_RELATIVE_SHOT] {
            let file = tree.path().join(relative);
            assert!(
                file.is_file(),
                "each live tree keeps its own captures: {} is missing",
                file.display(),
            );
        }
    }
}
