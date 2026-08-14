use std::{
    fs,
    path::PathBuf,
    process,
    sync::atomic::{AtomicU32, Ordering},
    time::SystemTime,
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

const ATTACHED_SHOT: &str = "target/qa_screenshots/attach_attached.png";

const SECOND_SHOT: &str = "target/qa_screenshots/attach_second.png";

const ATTACHED_BYTES: &[u8] = b"attach-attached-png-bytes";

const SECOND_BYTES: &[u8] = b"attach-second-png-bytes";

const ATTACHING_REPLY: &str = "(phase:(app:Running))";

struct AttachingLink(Vec<ReplyAttachment>);

impl QaLink for AttachingLink {
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        match request {
            QaRequest::Run(_) => Ok(QaResponse::Outcome(CommandOutcome::Ran {
                reply:       CommandReplyRon::new(ATTACHING_REPLY.to_owned()),
                attachments: self.0.clone(),
            })),
            _ => Ok(QaResponse::Error(QaError::Malformed)),
        }
    }
}

struct ChildInDirLifecycle(Option<PathBuf>);

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

    fn reap_dead_child(&mut self) {}

    fn child_working_dir(&self) -> Option<WorkingDir> {
        self.0.clone().map(WorkingDir::new)
    }

    fn child_output(&self, _max: TailLines) -> Option<OutputTail> {
        None
    }
}

static TREE_SERIAL: AtomicU32 = AtomicU32::new(0);

fn a_child_tree_holding_the_captures() -> PathBuf {
    let nanos = SystemTime::UNIX_EPOCH
        .elapsed()
        .map_or(0, |since| since.as_nanos());
    let serial = TREE_SERIAL.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("attach-{}-{nanos}-{serial}", process::id()));
    for (relative, bytes) in [(ATTACHED_SHOT, ATTACHED_BYTES), (SECOND_SHOT, SECOND_BYTES)] {
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
        !here.join(ATTACHED_SHOT).exists(),
        "the capture must not exist under the HOST's directory, or a host-relative read \
         would pass for the wrong reason",
    );
    root
}

fn png(path: &str) -> ReplyAttachment {
    ReplyAttachment::new(AttachmentKind::Png, ArtifactPath::new(path.to_owned()))
}

fn run_attaching(attachments: Vec<ReplyAttachment>, child_dir: Option<PathBuf>) -> Value {
    let (mut game_link, mut editor_link) = (AttachingLink(attachments), AttachingLink(Vec::new()));
    let (mut game_life, mut editor_life) = (
        ChildInDirLifecycle(child_dir.clone()),
        ChildInDirLifecycle(child_dir),
    );
    let mut hosts = HostSet::new(
        HostPair::new(&mut game_link, &mut game_life),
        HostPair::new(&mut editor_link, &mut editor_life),
    );
    let line = r#"{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{"name":"run","arguments":{"host":"game","command":"app.capture","arguments":"()"}}}"#;
    let Some(response) = dispatch(line, &mut hosts) else {
        unreachable!("a request with an id yields a response line");
    };
    serde_json::from_str(&response).unwrap_or(Value::Null)
}

#[test]
fn an_attached_capture_comes_back_as_an_image_read_under_the_childs_directory() {
    let root = a_child_tree_holding_the_captures();
    let response = run_attaching(vec![png(ATTACHED_SHOT)], Some(root.clone()));

    assert_eq!(
        response["result"]["isError"],
        json!(false),
        "a command that ran is a success: {response}",
    );
    assert_eq!(
        response["result"]["content"][0]["type"],
        json!("text"),
        "the reply's own body stays the FIRST block: {response}",
    );
    assert_eq!(response["result"]["content"][1]["type"], json!("image"));
    assert_eq!(
        response["result"]["content"][1]["mimeType"],
        json!("image/png")
    );
    assert_eq!(
        response["result"]["content"][1]["data"],
        json!(encode_standard(ATTACHED_BYTES)),
        "the image must be the bytes the CHILD's file holds: {response}",
    );
    drop(fs::remove_dir_all(&root));
}

#[test]
fn attachments_come_back_in_the_order_the_host_declared() {
    let root = a_child_tree_holding_the_captures();
    let response = run_attaching(
        vec![png(SECOND_SHOT), png(ATTACHED_SHOT)],
        Some(root.clone()),
    );

    assert_eq!(
        response["result"]["content"][1]["data"],
        json!(encode_standard(SECOND_BYTES)),
        "the first declared attachment is the first image block: {response}",
    );
    assert_eq!(
        response["result"]["content"][2]["data"],
        json!(encode_standard(ATTACHED_BYTES)),
        "the second declared attachment follows it: {response}",
    );
    drop(fs::remove_dir_all(&root));
}

#[test]
fn an_attachment_the_host_cannot_read_names_both_paths_and_emits_no_image() {
    let root = a_child_tree_holding_the_captures();
    let empty = root.join("no_captures_here");
    let Ok(()) = fs::create_dir_all(&empty) else {
        unreachable!("the test can create an empty child directory");
    };
    let response = run_attaching(vec![png(ATTACHED_SHOT)], Some(empty.clone()));

    let Some(text) = response["result"]["content"][1]["text"].as_str() else {
        unreachable!("an unreadable attachment renders a text block: {response}");
    };
    assert!(text.contains(ATTACHED_SHOT), "rendered: {text}");
    let opened = empty.join(ATTACHED_SHOT);
    let Some(shown) = opened.to_str() else {
        unreachable!("the child's path is valid UTF-8");
    };
    assert!(
        text.contains(shown),
        "the message names the path the host actually opened: {text}",
    );
    assert!(
        response["result"]["content"]
            .as_array()
            .is_some_and(|blocks| blocks.iter().all(|block| block["type"] != json!("image"))),
        "an unreadable attachment must never render as a fabricated image: {response}",
    );
    drop(fs::remove_dir_all(&root));
}
