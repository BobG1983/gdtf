use std::{
    fs,
    path::{Path, PathBuf},
    process,
};

use cobalt_mcp_protocol::{
    command::{ArtifactPath, AttachmentKind, CommandOutcome, CommandReplyRon, ReplyAttachment},
    message::{McpRequest, McpResponse, McpSessionError},
};
use cobalt_mcp_server::{
    ChildPid, HostLifecycle, InstanceId, LaunchOutcome, LaunchSpec, McpError, McpLink, McpPort,
    OutputTail, RecordedInstance, StopOutcome, TailLines, WorkingDir, base64::encode_standard,
    dispatch,
};
use serde_json::{Value, json};
use tempfile::TempDir;

use crate::jsonrpc::hosts::{test_identity, two_host_set};

const THISTLE_RELATIVE_SHOT: &str = "target/qa_screenshots/shotcwd_thistle.png";

const BRAMBLE_RELATIVE_SHOT: &str = "target/bramble_qa_screenshots/shotcwd_bramble.png";

const THISTLE_SHOT_BYTES: &[u8] = b"shotcwd-thistle-png-bytes";

const BRAMBLE_SHOT_BYTES: &[u8] = b"shotcwd-bramble-png-bytes";

const SECOND_BRAMBLE_SHOT_BYTES: &[u8] = b"shotcwd-second-bramble-png-bytes";

const CAPTURE_REPLY: &str = "()";

const THISTLE_ID: &str = "thistle-one";

const FIRST_BRAMBLE_ID: &str = "bramble-one";

const SECOND_BRAMBLE_ID: &str = "bramble-two";

const THISTLE_PORT: u16 = 4100;

const FIRST_BRAMBLE_PORT: u16 = 4200;

const SECOND_BRAMBLE_PORT: u16 = 4300;

struct RelativeShotLink(&'static str);

impl McpLink for RelativeShotLink {
    fn request(&mut self, request: McpRequest) -> Result<McpResponse, McpError> {
        match request {
            McpRequest::Run(_) => Ok(McpResponse::Outcome(CommandOutcome::Ran {
                reply:       CommandReplyRon::new(CAPTURE_REPLY.to_owned()),
                attachments: vec![ReplyAttachment::new(
                    AttachmentKind::Png,
                    ArtifactPath::new(self.0.to_owned()),
                )],
            })),
            _ => Ok(McpResponse::Error(McpSessionError::Malformed)),
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
            McpPort::new(self.port),
            ChildPid::new(process::id()),
        )
    }

    fn working_dir(&self) -> WorkingDir {
        WorkingDir::new(self.root.clone())
    }
}

struct ChildInDirLifecycle(Vec<SeededRoot>);

impl HostLifecycle for ChildInDirLifecycle {
    fn launch(&mut self, port: McpPort, _spec: &LaunchSpec) -> LaunchOutcome {
        LaunchOutcome::Launched {
            port,
            pid: ChildPid::new(process::id()),
            instance: InstanceId::new("instance-1".to_owned()),
        }
    }

    fn stop(&mut self, _port: McpPort) -> StopOutcome {
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
/// `bramble_bytes` is what this tree's bramble capture holds, so two trees differ by their
/// bramble capture alone.
fn a_child_tree_holding_both_captures(bramble_bytes: &[u8]) -> TempDir {
    let Ok(tree) = TempDir::new() else {
        unreachable!("the test can create a unique temp directory");
    };
    let root = tree.path();
    for (relative, bytes) in [
        (THISTLE_RELATIVE_SHOT, THISTLE_SHOT_BYTES),
        (BRAMBLE_RELATIVE_SHOT, bramble_bytes),
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
        !here.join(THISTLE_RELATIVE_SHOT).exists() && !here.join(BRAMBLE_RELATIVE_SHOT).exists(),
        "neither capture may exist under the HOST's directory, or a host-relative read \
         would pass for the wrong reason",
    );
    tree
}

fn dispatch_with_child_in(
    line: &str,
    bramble_roots: Vec<SeededRoot>,
    thistle_root: &Path,
) -> Value {
    let (mut thistle_link, mut bramble_link) = (
        RelativeShotLink(THISTLE_RELATIVE_SHOT),
        RelativeShotLink(BRAMBLE_RELATIVE_SHOT),
    );
    let (mut thistle_life, mut bramble_life) = (
        ChildInDirLifecycle(vec![SeededRoot::new(
            THISTLE_ID,
            THISTLE_PORT,
            thistle_root,
        )]),
        ChildInDirLifecycle(bramble_roots),
    );
    let mut hosts = two_host_set(
        &mut thistle_link,
        &mut thistle_life,
        &mut bramble_link,
        &mut bramble_life,
    );
    let Some(response) = dispatch(line, &test_identity(), &mut hosts) else {
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
fn a_thistle_capture_from_another_directory_relays_as_an_image() {
    let tree = a_child_tree_holding_both_captures(BRAMBLE_SHOT_BYTES);
    let response = dispatch_with_child_in(
        r#"{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"run","arguments":{"command":"sample.snapshot"}}}"#,
        Vec::new(),
        tree.path(),
    );
    assert_image_of(&response, THISTLE_SHOT_BYTES);
}

#[test]
fn a_bramble_capture_from_another_directory_relays_as_an_image() {
    let tree = a_child_tree_holding_both_captures(BRAMBLE_SHOT_BYTES);
    let response = dispatch_with_child_in(
        &format!(
            r#"{{"jsonrpc":"2.0","id":41,"method":"tools/call","params":{{"name":"run","arguments":{{"command":"sample.snapshot","host":"bramble","instance":"{FIRST_BRAMBLE_ID}"}}}}}}"#
        ),
        vec![SeededRoot::new(
            FIRST_BRAMBLE_ID,
            FIRST_BRAMBLE_PORT,
            tree.path(),
        )],
        tree.path(),
    );
    assert_image_of(&response, BRAMBLE_SHOT_BYTES);
}

#[test]
fn a_bramble_capture_is_read_from_the_tree_of_the_instance_the_call_names() {
    let first = a_child_tree_holding_both_captures(BRAMBLE_SHOT_BYTES);
    let second = a_child_tree_holding_both_captures(SECOND_BRAMBLE_SHOT_BYTES);

    // The call names the older record, so reading the newest one relays the other tree.
    let response = dispatch_with_child_in(
        &format!(
            r#"{{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{{"name":"run","arguments":{{"command":"sample.snapshot","host":"bramble","instance":"{FIRST_BRAMBLE_ID}"}}}}}}"#
        ),
        vec![
            SeededRoot::new(FIRST_BRAMBLE_ID, FIRST_BRAMBLE_PORT, first.path()),
            SeededRoot::new(SECOND_BRAMBLE_ID, SECOND_BRAMBLE_PORT, second.path()),
        ],
        first.path(),
    );

    assert_ne!(
        BRAMBLE_SHOT_BYTES, SECOND_BRAMBLE_SHOT_BYTES,
        "the two trees hold different bramble captures, or this proves nothing",
    );
    assert_image_of(&response, BRAMBLE_SHOT_BYTES);
}

#[test]
fn a_missing_capture_is_still_reported_naming_both_paths() {
    let tree = a_child_tree_holding_both_captures(BRAMBLE_SHOT_BYTES);
    let file = tree.path().join(THISTLE_RELATIVE_SHOT);
    let Ok(()) = fs::remove_file(&file) else {
        unreachable!("the test can delete the child's capture");
    };

    let response = dispatch_with_child_in(
        r#"{"jsonrpc":"2.0","id":43,"method":"tools/call","params":{"name":"run","arguments":{"command":"sample.snapshot"}}}"#,
        Vec::new(),
        tree.path(),
    );
    let Some(text) = response["result"]["content"][1]["text"].as_str() else {
        unreachable!("an unreadable attachment renders as a text block: {response}");
    };
    assert!(text.contains(THISTLE_RELATIVE_SHOT), "rendered: {text}");
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
