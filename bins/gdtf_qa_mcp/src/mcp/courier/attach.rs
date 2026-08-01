//! The files a command's reply hands back, as MCP content blocks (GTW-942).
//!
//! This is how a capture comes back WITHOUT the courier knowing what a capture is: the
//! command declares an attachment, the courier reads the file the child wrote and emits the
//! block its kind calls for. Adding a command that attaches a PNG needs no edit here.

use gdtf_qa_protocol::command::{AttachmentKind, ReplyAttachment};
use serde_json::{Value, json};

use crate::{
    base64::encode_standard,
    lifecycle::WorkingDir,
    mcp::child_path::{ChildReportedPath, resolve_child_path},
};

/// One attachment as an MCP content block, or a text block naming why it could not be read.
///
/// NEVER a fabricated image: a file the courier could not open is reported, naming BOTH the
/// path the child gave and the path the host actually opened, because a directory mismatch
/// is invisible from either one alone (GTW-923).
fn attachment_block(attachment: &ReplyAttachment, child_dir: Option<&WorkingDir>) -> Value {
    let resolved = resolve_child_path(ChildReportedPath::from(&attachment.path), child_dir);
    match (attachment.kind, std::fs::read(&*resolved)) {
        (AttachmentKind::Png, Ok(bytes)) => json!({
            "type": "image",
            "data": encode_standard(&bytes),
            "mimeType": "image/png",
        }),
        (AttachmentKind::Png, Err(err)) => json!({
            "type": "text",
            "text": format!(
                "the command attached {} (read as {}) but it could not be read: {err}",
                attachment.path.as_str(),
                resolved.display(),
            ),
        }),
    }
}

/// Every attachment on a reply, as the content blocks that follow the reply's own text.
///
/// Order is the host's: a command that attaches a before and an after capture gets them
/// back in the order it declared them.
pub(super) fn attachment_blocks(
    attachments: &[ReplyAttachment],
    child_dir: Option<&WorkingDir>,
) -> Vec<Value> {
    attachments
        .iter()
        .map(|attachment| attachment_block(attachment, child_dir))
        .collect()
}
