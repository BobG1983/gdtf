//! Attachment draft conversion and RON save helpers.

use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_battle_sim::equipment::attachments::{AttachmentName, AttachmentSpec};
use gdtf_content_families::AttachmentsFamily;

use super::draft::AttachmentDraft;

/// Build an attachment name and spec from a draft.
#[must_use]
pub fn draft_to_attachment_spec(draft: &AttachmentDraft) -> (AttachmentName, AttachmentSpec) {
    let name = AttachmentName::new(draft.name().trim().to_owned());
    (name, draft.spec().clone())
}

/// File name for an attachment asset under the attachments family folder.
#[must_use]
pub fn attachment_file_name(name: &AttachmentName) -> String {
    let stem = sanitize_file_stem(name.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed attachment")
    } else {
        stem
    };
    format!("{stem}.{}", AttachmentsFamily::EXTENSION)
}

/// Full path for an attachment asset under `root`.
#[must_use]
pub fn attachment_save_path_in(root: &Path, name: &AttachmentName) -> PathBuf {
    root.join(AttachmentsFamily::FOLDER)
        .join(attachment_file_name(name))
}

/// Write attachment RON under `root`.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_attachment_in(
    root: &Path,
    name: &AttachmentName,
    spec: &AttachmentSpec,
) -> Result<PathBuf, RonSaveError> {
    let path = attachment_save_path_in(root, name);
    write_ron_pretty(&path, spec)?;
    Ok(path)
}

/// Write attachment RON under the workspace assets root.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_attachment(
    name: &AttachmentName,
    spec: &AttachmentSpec,
) -> Result<PathBuf, RonSaveError> {
    write_attachment_in(Path::new(WORKSPACE_ASSETS_ROOT), name, spec)
}
