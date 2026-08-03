use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_battle_sim::equipment::attachments::{AttachmentName, AttachmentSpec};
use gdtf_content_families::AttachmentsFamily;

use super::draft::AttachmentDraft;

#[must_use]
pub fn draft_to_attachment_spec(draft: &AttachmentDraft) -> (AttachmentName, AttachmentSpec) {
    let name = AttachmentName::new(draft.name().trim().to_owned());
    (name, draft.spec().clone())
}

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

#[must_use]
pub fn attachment_save_path_in(root: &Path, name: &AttachmentName) -> PathBuf {
    root.join(AttachmentsFamily::FOLDER)
        .join(attachment_file_name(name))
}

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

#[cfg(debug_assertions)]
pub fn write_attachment(
    name: &AttachmentName,
    spec: &AttachmentSpec,
) -> Result<PathBuf, RonSaveError> {
    write_attachment_in(Path::new(WORKSPACE_ASSETS_ROOT), name, spec)
}
