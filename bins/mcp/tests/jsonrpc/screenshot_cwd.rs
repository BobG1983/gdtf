use std::{
    fs,
    path::{Path, PathBuf},
    process,
};

use cobalt_mcp_protocol::{
    command::{ArtifactPath, AttachmentKind, CommandOutcome, CommandReplyRon, ReplyAttachment},
    message::{QaError, QaRequest, QaResponse},
};
use mcp::{
    ChildPid, HostLifecycle, HostPair, HostSet, InstanceId, LaunchOutcome, LaunchSpec, McpError,
    OutputTail, QaLink, QaPort, RecordedInstance, StopOutcome, TailLines, WorkingDir,
    base64::encode_standard, dispatch,
};
use serde_json::{Value, json};
use tempfile::TempDir;

const GAME_RELATIVE_SHOT: &str = "target/qa_screenshots/shotcwd_game.png";

const EDITOR_RELATIVE_SHOT: &str = "target/editor_qa_screenshots/shotcwd_editor.png";

const GAME_SHOT_BYTES: &[u8] = b"shotcwd-game-png-bytes";

const EDITOR_SHOT_BYTES: &[u8] = b"shotcwd-editor-png-bytes";

const SECOND_EDITOR_SHOT_BYTES: &[u8] = b"shotcwd-second-editor-png-bytes";

const CAPTURE_REPLY: &str = "()";

const GAME_ID: &str = "game-one";

const FIRST_EDITOR_ID: &str = "editor-one";

const SECOND_EDITOR_ID: &str = "editor-two";

const GAME_PORT: u16 = 7616;

const FIRST_EDITOR_PORT: u16 = 7617;

const SECOND_EDITOR_PORT: u16 = 7618;

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

/// One instance the fixture records, with the tree that child ran in.
struct SeededRoot {
    id:   &'static str,
    port: u16,
    root: PathBuf,
}

impl SeededRoot {
    fn new(id: &'static str, port: u16, root: &Path) -> Self {
        Self {
            id,
            port,
            root: root.to_path_buf(),
        }
    }

    fn recorded(&self) -> RecordedInstance {
        RecordedInstance::new(
            InstanceId::new(self.id.to_owned()),
            QaPort::new(self.port),
            ChildPid::new(process::id()),
        )
    }

    fn working_dir(&self) -> WorkingDir {
        WorkingDir::new(self.root.clone())
    }
}

struct ChildInDirLifecycle(Vec<SeededRoot>);

impl HostLifecycle for ChildInDirLifecycle {
    fn launch(&mut self, port: QaPort, _spec: &LaunchSpec) -> LaunchOutcome {
        LaunchOutcome::Launched {
            port,
            pid: ChildPid::new(process::id()),
            instance: InstanceId::new("instance-1".to_owned()),
        }
    }

    fn stop(&mut self, _port: QaPort) -> StopOutcome {
        StopOutcome::NotRunning
    }

    fn stop_instance(&mut self, _instance: &InstanceId) -> StopOutcome {
        StopOutcome::NotRunning
    }

    fn stop_owned(&mut self) -> StopOutcome {
        StopOutcome::NotRunning
    }

    fn reap_dead_child(&mut self) {}

    fn instances(&self) -> Vec<RecordedInstance> {
        self.0.iter().map(SeededRoot::recorded).collect()
    }

    fn child_working_dir(&self) -> Option<WorkingDir> {
        self.0.last().map(SeededRoot::working_dir)
    }

    fn instance_working_dir(&self, instance: &InstanceId) -> Option<WorkingDir> {
        self.0
            .iter()
            .find(|seeded| seeded.id == instance.as_str())
            .map(SeededRoot::working_dir)
    }

    fn child_output(&self, _max: TailLines) -> Option<OutputTail> {
        None
    }

    fn instance_output(&self, _instance: &InstanceId, _max: TailLines) -> Option<OutputTail> {
        None
    }
}

/// Own a unique temp root so parallel tests in this process cannot delete each other.
///
/// `editor_bytes` is what this tree's editor capture holds, so two trees differ by their
/// editor capture alone.
fn a_child_tree_holding_both_captures(editor_bytes: &[u8]) -> TempDir {
    let Ok(tree) = TempDir::new() else {
        unreachable!("the test can create a unique temp directory");
    };
    let root = tree.path();
    for (relative, bytes) in [
        (GAME_RELATIVE_SHOT, GAME_SHOT_BYTES),
        (EDITOR_RELATIVE_SHOT, editor_bytes),
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
    assert_ne!(
        root,
        here.as_path(),
        "the child's tree differs from the host's"
    );
    assert!(
        !here.join(GAME_RELATIVE_SHOT).exists() && !here.join(EDITOR_RELATIVE_SHOT).exists(),
        "neither capture may exist under the HOST's directory, or a host-relative read \
         would pass for the wrong reason",
    );
    tree
}

fn dispatch_with_child_in(line: &str, editor_roots: Vec<SeededRoot>, game_root: &Path) -> Value {
    let (mut game_link, mut editor_link) = (
        RelativeShotLink(GAME_RELATIVE_SHOT),
        RelativeShotLink(EDITOR_RELATIVE_SHOT),
    );
    let (mut game_life, mut editor_life) = (
        ChildInDirLifecycle(vec![SeededRoot::new(GAME_ID, GAME_PORT, game_root)]),
        ChildInDirLifecycle(editor_roots),
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
    let tree = a_child_tree_holding_both_captures(EDITOR_SHOT_BYTES);
    let response = dispatch_with_child_in(
        r#"{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"run","arguments":{"command":"capture.screenshot"}}}"#,
        Vec::new(),
        tree.path(),
    );
    assert_image_of(&response, GAME_SHOT_BYTES);
}

#[test]
fn an_editor_capture_from_another_directory_relays_as_an_image() {
    let tree = a_child_tree_holding_both_captures(EDITOR_SHOT_BYTES);
    let response = dispatch_with_child_in(
        &format!(
            r#"{{"jsonrpc":"2.0","id":41,"method":"tools/call","params":{{"name":"run","arguments":{{"command":"capture.screenshot","host":"editor","instance":"{FIRST_EDITOR_ID}"}}}}}}"#
        ),
        vec![SeededRoot::new(
            FIRST_EDITOR_ID,
            FIRST_EDITOR_PORT,
            tree.path(),
        )],
        tree.path(),
    );
    assert_image_of(&response, EDITOR_SHOT_BYTES);
}

#[test]
fn an_editor_capture_is_read_from_the_tree_of_the_instance_the_call_names() {
    let first = a_child_tree_holding_both_captures(EDITOR_SHOT_BYTES);
    let second = a_child_tree_holding_both_captures(SECOND_EDITOR_SHOT_BYTES);

    // The call names the older record, so reading the newest one relays the other tree.
    let response = dispatch_with_child_in(
        &format!(
            r#"{{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{{"name":"run","arguments":{{"command":"capture.screenshot","host":"editor","instance":"{FIRST_EDITOR_ID}"}}}}}}"#
        ),
        vec![
            SeededRoot::new(FIRST_EDITOR_ID, FIRST_EDITOR_PORT, first.path()),
            SeededRoot::new(SECOND_EDITOR_ID, SECOND_EDITOR_PORT, second.path()),
        ],
        first.path(),
    );

    assert_ne!(
        EDITOR_SHOT_BYTES, SECOND_EDITOR_SHOT_BYTES,
        "the two trees hold different editor captures, or this proves nothing",
    );
    assert_image_of(&response, EDITOR_SHOT_BYTES);
}

#[test]
fn a_missing_capture_is_still_reported_naming_both_paths() {
    let tree = a_child_tree_holding_both_captures(EDITOR_SHOT_BYTES);
    let file = tree.path().join(GAME_RELATIVE_SHOT);
    let Ok(()) = fs::remove_file(&file) else {
        unreachable!("the test can delete the child's capture");
    };

    let response = dispatch_with_child_in(
        r#"{"jsonrpc":"2.0","id":43,"method":"tools/call","params":{"name":"run","arguments":{"command":"capture.screenshot"}}}"#,
        Vec::new(),
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
