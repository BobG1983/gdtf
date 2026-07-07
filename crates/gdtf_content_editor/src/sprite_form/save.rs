//! The SPRITE-mode form's **projection + path + write** (GTW-664 C1): turn the
//! [`SpriteDraft`] into the loader's `(`[`SpriteName`]`, `[`SpriteDef`]`)` pair, resolve
//! the one-owner save location, and write it through the shared RON writer so the
//! GTW-663 [`SpriteDefsFamily`] folder loader reads back exactly what was saved.
//!
//! Every path segment is DERIVED, never re-spelled (GTW-621/634): the root is the shared
//! [`WORKSPACE_ASSETS_ROOT`] owner, the folder / compound extension come from
//! [`SpriteDefsFamily`]'s `FOLDER` / `EXTENSION`, and the stem runs through the shared
//! [`sanitize_file_stem`] seam. [`draft_to_sprite_def`] / [`sprite_file_name`] /
//! [`sprite_save_path_in`] are PURE (no IO) so tests can round-trip them without
//! touching the assets tree; the filesystem write lives in [`write_sprite_in`]
//! (root-parameterized — the GTW-555 `write_terrain_in` precedent, so tests aim it at a
//! `TempDir`) and its thin production wrapper [`write_sprite`] (both debug-only, the
//! gang / armor save precedent).
//!
//! RE-VALIDATION IS FREE (GTW-664 C4, cited not rebuilt): a save overwrites the member
//! on disk, the family redrive rebuilds the [`SpriteDefRegistry`](gdtf_content_families::sprites::SpriteDefRegistry)
//! from it, and the editor's validation re-arm watch set (`validate/rearm.rs`,
//! `WatchedRegistries::sprite_defs` — GTW-663) already watches that registry — so the
//! terrain `graphic_name` integrity edge re-runs live on every sprite save with zero new
//! machinery here.

use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_content_families::sprites::{SpriteDef, SpriteDefsFamily, SpriteName};

use super::draft::SpriteDraft;

/// Project the in-progress [`SpriteDraft`] into the loader's `(`[`SpriteName`]`,
/// `[`SpriteDef`]`)` pair — the EXACT schema the GTW-663 sprite-defs loader reads (the
/// GTW-636 round-trip contract), never a parallel one. The name buffer folds (trimmed)
/// into the registry-key [`SpriteName`]; the def is the family record itself, so the
/// projection is a copy.
///
/// Pure (no IO) so the round-trip tests project through the SAME conversion the Save
/// button runs.
#[must_use]
pub fn draft_to_sprite_def(draft: &SpriteDraft) -> (SpriteName, SpriteDef) {
    let name = SpriteName::new(draft.name().trim().to_owned());
    (name, draft.def().clone())
}

/// The on-disk FILE NAME for a saved sprite def — `<sanitized_name>.spritedef.ron`.
///
/// The suffix is DERIVED from [`SpriteDefsFamily::EXTENSION`] (the ONE canonical
/// extension discriminant the sprite-defs folder loader dispatches on — GTW-621: a
/// re-spelled extension drifts and every saved file silently vanishes on reload). The
/// stem runs through the shared [`sanitize_file_stem`] seam (GTW-577) so a path-hostile
/// sprite name can never reach the filesystem raw; a name that sanitizes to NOTHING
/// falls back to the documented `unnamed_sprite` stem (minted through the SAME seam —
/// the gang / armor save parity). The loader keys a sprite by its file stem, so a saved
/// def reloads keyed by exactly its sanitized stem.
///
/// Pure (no IO) so a test can assert the resolved name without writing anything.
#[must_use]
pub fn sprite_file_name(name: &SpriteName) -> String {
    let stem = sanitize_file_stem(name.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed sprite")
    } else {
        stem
    };
    format!("{stem}.{}", SpriteDefsFamily::EXTENSION)
}

/// The full on-disk PATH a saved sprite def is written to under an arbitrary assets
/// `root`: `<root>/`[`SpriteDefsFamily::FOLDER`]`/` joined with the [`sprite_file_name`]
/// — the root-parameterized core (the GTW-555 pattern), so a test resolves the REAL save
/// location against a `TempDir` root instead of the version-controlled `assets/` tree.
///
/// Pure (no IO) so a test can assert the resolved location without writing anything.
#[must_use]
pub fn sprite_save_path_in(root: &Path, name: &SpriteName) -> PathBuf {
    root.join(SpriteDefsFamily::FOLDER)
        .join(sprite_file_name(name))
}

/// Serialize + WRITE a sprite def under an arbitrary assets `root` — the
/// root-parameterized write core (the GTW-555 `write_terrain_in` precedent): the GTW-664
/// round-trip test drives THIS real write into a `TempDir` assets root and loads it back
/// through the REAL sprite-defs folder walk, never polluting the shipped `assets/` tree.
///
/// Resolves the sanitized path ([`sprite_save_path_in`]) and hands the serialize →
/// mkdir → write chain to the shared [`write_ron_pretty`] seam (GTW-577 C2). Returns the
/// written path on success so the caller can log it. Debug-only (the terrain / theme /
/// gang / armor save precedent): the fs write never compiles into a release binary.
///
/// # Errors
///
/// The seam's [`RonSaveError`], whose `Display` names the failed stage (serialize vs
/// write).
#[cfg(debug_assertions)]
pub fn write_sprite_in(
    root: &Path,
    name: &SpriteName,
    def: &SpriteDef,
) -> Result<PathBuf, RonSaveError> {
    let path = sprite_save_path_in(root, name);
    write_ron_pretty(&path, def)?;
    Ok(path)
}

/// Write a sprite def to the workspace `assets/` tree — [`write_sprite_in`] under the
/// shared [`WORKSPACE_ASSETS_ROOT`] owner (byte-identical to the app's
/// `AssetPlugin.file_path` by construction), so the saved def lands exactly where the
/// GTW-663 folder loader reads. The thin root-supplying wrapper the Save button calls.
///
/// # Errors
///
/// The seam's [`RonSaveError`] (see [`write_sprite_in`]).
#[cfg(debug_assertions)]
pub fn write_sprite(name: &SpriteName, def: &SpriteDef) -> Result<PathBuf, RonSaveError> {
    write_sprite_in(Path::new(WORKSPACE_ASSETS_ROOT), name, def)
}
