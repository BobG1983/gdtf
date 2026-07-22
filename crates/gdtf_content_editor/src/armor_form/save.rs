//! The ARMOR-mode form's **projection + path + write** (GTW-479 C1): turn the
//! [`ArmorDraft`] into the loader's `(`[`ArmorName`]`, `[`ArmorSpec`]`)` pair, resolve
//! the one-owner save location, and write it through the shared RON writer so the
//! GTW-269 armor folder loader reads back exactly what was saved.
//!
//! Every path segment is DERIVED, never re-spelled (GTW-621/634): the root is the shared
//! [`WORKSPACE_ASSETS_ROOT`] owner, the folder / compound extension come from
//! [`ArmorFamily`]'s `FOLDER` / `EXTENSION`, and the stem runs through the shared
//! [`sanitize_file_stem`] helper. [`draft_to_spec`] / [`armor_file_name`] /
//! [`armor_save_path_in`] are PURE (no IO) so tests can round-trip them without touching
//! the assets tree; the filesystem write lives in [`write_armor_in`] (root-parameterized
//! — the GTW-555 `write_terrain_in` precedent, so tests aim it at a `TempDir`) and its
//! thin production wrapper [`write_armor`] (both debug-only, the gang save precedent).

use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_battle_sim::armor::{ArmorName, ArmorSpec};
use gdtf_content_families::ArmorFamily;

use super::draft::ArmorDraft;

/// Project the in-progress [`ArmorDraft`] into the loader's `(`[`ArmorName`]`,
/// `[`ArmorSpec`]`)` pair — the EXACT schema the GTW-269 armor loader reads (the
/// GTW-636 round-trip contract), never a parallel one. The name buffer folds (trimmed)
/// into the registry-key [`ArmorName`]; the suit is the sim record itself, so the
/// projection is a copy.
///
/// Pure (no IO) so the round-trip tests project through the SAME conversion the Save
/// button runs.
#[must_use]
pub fn draft_to_spec(draft: &ArmorDraft) -> (ArmorName, ArmorSpec) {
    let name = ArmorName::new(draft.name().trim().to_owned());
    (name, *draft.spec())
}

/// The on-disk FILE NAME for a saved armor suit — `<sanitized_armor_name>.armor.ron`.
///
/// The suffix is DERIVED from [`ArmorFamily::EXTENSION`] (the ONE canonical extension
/// discriminant the armor folder loader dispatches on — GTW-621: a re-spelled extension
/// drifts and every saved file silently vanishes on reload). The stem runs through the
/// shared [`sanitize_file_stem`] helper (GTW-577) so a path-hostile armor name can never
/// reach the filesystem raw; a name that sanitizes to NOTHING falls back to the
/// documented `unnamed_armor` stem (minted through the SAME helper — the gang save's
/// `unnamed_gang` parity). The loader keys an armor by its file stem with the `.armor`
/// infix stripped, so a saved suit reloads keyed by exactly its sanitized stem.
///
/// Pure (no IO) so a test can assert the resolved name without writing anything.
#[must_use]
pub fn armor_file_name(name: &ArmorName) -> String {
    let stem = sanitize_file_stem(name.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed armor")
    } else {
        stem
    };
    format!("{stem}.{}", ArmorFamily::EXTENSION)
}

/// The full on-disk PATH a saved armor suit is written to under an arbitrary assets
/// `root`: `<root>/`[`ArmorFamily::FOLDER`]`/` joined with the [`armor_file_name`] — the
/// root-parameterized core (the GTW-555 pattern), so a test resolves the REAL save
/// location against a `TempDir` root instead of the version-controlled `assets/` tree.
///
/// Pure (no IO) so a test can assert the resolved location without writing anything.
#[must_use]
pub fn armor_save_path_in(root: &Path, name: &ArmorName) -> PathBuf {
    root.join(ArmorFamily::FOLDER).join(armor_file_name(name))
}

/// Serialize + WRITE an armor spec under an arbitrary assets `root` — the
/// root-parameterized write core (the GTW-555 `write_terrain_in` precedent): the GTW-479
/// round-trip test drives THIS real write into a `TempDir` assets root and loads it back
/// through the REAL armor folder walk, never polluting the shipped `assets/` tree.
///
/// Resolves the sanitized path ([`armor_save_path_in`]) and hands the serialize → mkdir
/// → write chain to the shared [`write_ron_pretty`] writer (GTW-577 C2). Returns the
/// written path on success so the caller can log it. Debug-only (the terrain / theme /
/// gang save precedent): the fs write never compiles into a release binary.
///
/// # Errors
///
/// The writer's [`RonSaveError`], whose `Display` names the failed stage (serialize vs
/// write).
#[cfg(debug_assertions)]
pub fn write_armor_in(
    root: &Path,
    name: &ArmorName,
    spec: &ArmorSpec,
) -> Result<PathBuf, RonSaveError> {
    let path = armor_save_path_in(root, name);
    write_ron_pretty(&path, spec)?;
    Ok(path)
}

/// Write an armor spec to the workspace `assets/` tree — [`write_armor_in`] under the
/// shared [`WORKSPACE_ASSETS_ROOT`] owner (identical to the app's
/// `AssetPlugin.file_path` by construction), so the saved suit lands exactly where the
/// GTW-269 folder loader reads. The thin root-supplying wrapper the Save button calls.
///
/// # Errors
///
/// The writer's [`RonSaveError`] (see [`write_armor_in`]).
#[cfg(debug_assertions)]
pub fn write_armor(name: &ArmorName, spec: &ArmorSpec) -> Result<PathBuf, RonSaveError> {
    write_armor_in(Path::new(WORKSPACE_ASSETS_ROOT), name, spec)
}
