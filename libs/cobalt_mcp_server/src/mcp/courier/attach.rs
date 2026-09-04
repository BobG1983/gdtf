use cobalt_mcp_protocol::command::{AttachmentKind, ReplyAttachment};
use serde_json::{Value, json};

use crate::{
    base64::encode_standard,
    lifecycle::WorkingDir,
    mcp::child_path::{ChildReportedPath, resolve_child_path},
};

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

pub(super) fn attachment_blocks(
    attachments: &[ReplyAttachment],
    child_dir: Option<&WorkingDir>,
) -> Vec<Value> {
    attachments
        .iter()
        .map(|attachment| attachment_block(attachment, child_dir))
        .collect()
}
