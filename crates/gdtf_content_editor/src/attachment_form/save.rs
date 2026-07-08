//! The ATTACHMENT-mode form's **projection + path + write** (GTW-669 C3): turn the
//! [`AttachmentDraft`] into the loader's `(`[`AttachmentName`]`, `[`AttachmentSpec`]`)`
//! pair, resolve the one-owner save location, and write it through the shared RON writer
//! so the GTW-619 [`AttachmentsFamily`] folder loader reads back exactly what was saved.
//!
//! Every path segment is DERIVED, never re-spelled (GTW-621/634): the root is the shared
//! [`WORKSPACE_ASSETS_ROOT`] owner, the folder / compound extension come from
//! [`AttachmentsFamily`]'s `FOLDER` / `EXTENSION`, and the stem runs through the shared
//! [`sanitize_file_stem`] seam. [`draft_to_attachment_spec`] / [`attachment_file_name`] /
//! [`attachment_save_path_in`] are PURE (no IO) so tests can round-trip them without
//! touching the assets tree; the filesystem write lives in [`write_attachment_in`]
//! (root-parameterized — the GTW-555 `write_terrain_in` precedent, so tests aim it at a
//! `TempDir`) and its thin production wrapper [`write_attachment`] (both debug-only, the
//! gang / armor / sprite save precedent).
//!
//! RE-VALIDATION IS FREE (GTW-669 C4, cited not rebuilt): a save overwrites the member
//! on disk, the family redrive rebuilds the
//! [`AttachmentRegistry`](gdtf_battle_sim::equipment::attachments::AttachmentRegistry)
//! from it, and the editor's validation re-arm watch set (`validate/rearm.rs`,
//! `WatchedRegistries::attachments` — GTW-669) watches that registry — so the
//! weapon→attachment integrity edge re-runs live on every attachment save with zero new
//! machinery here.

use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_battle_sim::equipment::attachments::{AttachmentName, AttachmentSpec};
use gdtf_content_families::AttachmentsFamily;

use super::draft::AttachmentDraft;

/// Project the in-progress [`AttachmentDraft`] into the loader's `(`[`AttachmentName`]`,
/// `[`AttachmentSpec`]`)` pair — the EXACT schema the GTW-619 attachments loader reads
/// (the GTW-636 round-trip contract), never a parallel one. The name buffer folds
/// (trimmed) into the registry-key [`AttachmentName`]; the spec is the sim record
/// itself, so the projection is a copy.
///
/// Pure (no IO) so the round-trip tests project through the SAME conversion the Save
/// button runs.
#[must_use]
pub fn draft_to_attachment_spec(draft: &AttachmentDraft) -> (AttachmentName, AttachmentSpec) {
    let name = AttachmentName::new(draft.name().trim().to_owned());
    (name, draft.spec().clone())
}

/// The on-disk FILE NAME for a saved attachment item — `<sanitized_name>.attachment.ron`.
///
/// The suffix is DERIVED from [`AttachmentsFamily::EXTENSION`] (the ONE canonical
/// extension discriminant the attachments folder loader dispatches on — GTW-621: a
/// re-spelled extension drifts and every saved file silently vanishes on reload). The
/// stem runs through the shared [`sanitize_file_stem`] seam (GTW-577) so a path-hostile
/// item name can never reach the filesystem raw; a name that sanitizes to NOTHING falls
/// back to the documented `unnamed_attachment` stem (minted through the SAME seam — the
/// gang / armor / sprite save parity). The loader keys an item by its file stem, so a
/// saved item reloads keyed by exactly its sanitized stem.
///
/// Pure (no IO) so a test can assert the resolved name without writing anything.
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

/// The full on-disk PATH a saved attachment item is written to under an arbitrary assets
/// `root`: `<root>/`[`AttachmentsFamily::FOLDER`]`/` joined with the
/// [`attachment_file_name`] — the root-parameterized core (the GTW-555 pattern), so a
/// test resolves the REAL save location against a `TempDir` root instead of the
/// version-controlled `assets/` tree.
///
/// Pure (no IO) so a test can assert the resolved location without writing anything.
#[must_use]
pub fn attachment_save_path_in(root: &Path, name: &AttachmentName) -> PathBuf {
    root.join(AttachmentsFamily::FOLDER)
        .join(attachment_file_name(name))
}

/// Serialize + WRITE an attachment item under an arbitrary assets `root` — the
/// root-parameterized write core (the GTW-555 `write_terrain_in` precedent): the GTW-669
/// round-trip test drives THIS real write into a `TempDir` assets root and loads it back
/// through the REAL attachments folder walk, never polluting the shipped `assets/` tree.
///
/// Resolves the sanitized path ([`attachment_save_path_in`]) and hands the serialize →
/// mkdir → write chain to the shared [`write_ron_pretty`] seam (GTW-577 C2). Returns the
/// written path on success so the caller can log it. Debug-only (the terrain / theme /
/// gang / armor / sprite save precedent): the fs write never compiles into a release
/// binary.
///
/// # Errors
///
/// The seam's [`RonSaveError`], whose `Display` names the failed stage (serialize vs
/// write).
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

/// Write an attachment item to the workspace `assets/` tree — [`write_attachment_in`]
/// under the shared [`WORKSPACE_ASSETS_ROOT`] owner (byte-identical to the app's
/// `AssetPlugin.file_path` by construction), so the saved item lands exactly where the
/// GTW-619 folder loader reads. The thin root-supplying wrapper the Save button calls.
///
/// # Errors
///
/// The seam's [`RonSaveError`] (see [`write_attachment_in`]).
#[cfg(debug_assertions)]
pub fn write_attachment(
    name: &AttachmentName,
    spec: &AttachmentSpec,
) -> Result<PathBuf, RonSaveError> {
    write_attachment_in(Path::new(WORKSPACE_ASSETS_ROOT), name, spec)
}
