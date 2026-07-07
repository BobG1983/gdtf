//! The INJURY-mode forms' **projection + path + write** (GTW-654 C1/C2): turn the
//! [`InjuryDraft`] into the loader's `(`[`InjuryName`]` key, `[`InjuryDef`]`)` pair
//! and the [`WeightingDraft`] into the authored [`InjuryWeighting`], resolve the
//! one-owner save locations, and write both through the shared RON writer so the
//! bespoke GTW-437 injuries folder loader reads back exactly what was saved.
//!
//! Every path segment is DERIVED, never re-spelled (GTW-621/634): the root is the
//! shared [`WORKSPACE_ASSETS_ROOT`] owner; the folder / subfolders / compound
//! extensions come from the injuries family's one-owner layout consts
//! ([`INJURIES_FOLDER`] / [`category_dir`] / [`WEIGHTING_SUBFOLDER`] /
//! [`INJURY_DEF_EXTENSION`] / [`INJURY_WEIGHTING_EXTENSION`]); and the def stem
//! runs through the shared [`sanitize_file_stem`] seam. The projections and path
//! fns are PURE (no IO) so tests round-trip them without touching the assets tree;
//! the filesystem writes live in the root-parameterized `write_*_in` cores (the
//! GTW-555 `write_terrain_in` precedent, so tests aim them at a `TempDir`) and
//! their thin production wrappers (all debug-only, the gang/armor save precedent).

use std::path::{Path, PathBuf};

use gdtf_assets::sanitize_file_stem;
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{InjuryDef, InjuryName, InjuryWeighting},
};
use gdtf_content_families::injuries::{
    INJURIES_FOLDER, INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION, WEIGHTING_SUBFOLDER,
    category_dir,
};

use super::{draft::InjuryDraft, weighting::WeightingDraft};

/// Project the in-progress [`InjuryDraft`] into the loader's `(`[`InjuryName`]` key,
/// `[`InjuryDef`]`)` pair — the EXACT schema the GTW-437 injuries loader reads (the
/// GTW-636 round-trip contract), never a parallel one. The key buffer folds
/// (trimmed) into the stem-role [`InjuryName`]; the def is the sim record itself,
/// so the projection is a copy.
///
/// Pure (no IO) so the round-trip tests project through the SAME conversion the
/// Save button runs.
#[must_use]
pub fn draft_to_def(draft: &InjuryDraft) -> (InjuryName, InjuryDef) {
    let key = InjuryName::new(draft.key().trim().to_owned());
    (key, draft.def().clone())
}

/// Project the in-progress [`WeightingDraft`] into the loader's authored
/// [`InjuryWeighting`] schema — a copy of the sim record the draft holds.
///
/// Pure (no IO), the [`draft_to_def`] parity for the second artifact kind.
#[must_use]
pub fn draft_to_weighting(draft: &WeightingDraft) -> InjuryWeighting {
    draft.weighting().clone()
}

/// The on-disk FILE NAME for a saved injury def —
/// `<sanitized_key>.injury.ron`.
///
/// The compound suffix is DERIVED from [`INJURY_DEF_EXTENSION`] (the ONE canonical
/// extension discriminant the injuries loader dispatches on — GTW-621: a re-spelled
/// extension drifts and every saved file silently vanishes on reload). The stem
/// runs through the shared [`sanitize_file_stem`] seam (GTW-577) so a path-hostile
/// key can never reach the filesystem raw; a key that sanitizes to NOTHING falls
/// back to the documented `unnamed_injury` stem (minted through the SAME seam —
/// the gang/armor save parity). The loader keys an injury by its file stem with
/// the `.injury` infix stripped, so a saved def reloads keyed by exactly its
/// sanitized stem.
///
/// Pure (no IO) so a test can assert the resolved name without writing anything.
#[must_use]
pub fn injury_file_name(key: &InjuryName) -> String {
    let stem = sanitize_file_stem(key.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed injury")
    } else {
        stem
    };
    format!("{stem}.{INJURY_DEF_EXTENSION}")
}

/// The full on-disk PATH a saved injury def is written to under an arbitrary
/// assets `root`:
/// `<root>/`[`INJURIES_FOLDER`]`/`[`category_dir`]`(category)/` joined with the
/// [`injury_file_name`] — the def lands in its authored category's subfolder, so
/// the loader's organizational subfolder audit (GTW-440) never warns on an
/// editor-saved file. Root-parameterized (the GTW-555 pattern) so a test resolves
/// the REAL save location against a `TempDir` root.
///
/// Pure (no IO) so a test can assert the resolved location without writing.
#[must_use]
pub fn injury_save_path_in(root: &Path, category: InjuryCategory, key: &InjuryName) -> PathBuf {
    root.join(INJURIES_FOLDER)
        .join(category_dir(category))
        .join(injury_file_name(key))
}

/// The on-disk FILE NAME for a saved weighting table —
/// `<category_dir>.weighting.ron` (the shipped convention: one table per category,
/// stemmed by the category's canonical directory name — `head.weighting.ron` etc.).
/// Both halves derive from their one owners ([`category_dir`] /
/// [`INJURY_WEIGHTING_EXTENSION`]); the stem is a fixed enum projection, so no
/// sanitize pass is needed.
///
/// Pure (no IO) so a test can assert the resolved name without writing anything.
#[must_use]
pub fn weighting_file_name(category: InjuryCategory) -> String {
    format!("{}.{INJURY_WEIGHTING_EXTENSION}", category_dir(category))
}

/// The full on-disk PATH a saved weighting table is written to under an arbitrary
/// assets `root`: `<root>/`[`INJURIES_FOLDER`]`/`[`WEIGHTING_SUBFOLDER`]`/` joined
/// with the [`weighting_file_name`] — exactly where the shipped per-category
/// tables live. Root-parameterized (the GTW-555 pattern).
///
/// Pure (no IO) so a test can assert the resolved location without writing.
#[must_use]
pub fn weighting_save_path_in(root: &Path, category: InjuryCategory) -> PathBuf {
    root.join(INJURIES_FOLDER)
        .join(WEIGHTING_SUBFOLDER)
        .join(weighting_file_name(category))
}

/// Serialize + WRITE an injury def under an arbitrary assets `root` — the
/// root-parameterized write core (the GTW-555 `write_terrain_in` precedent): the
/// GTW-654 round-trip test drives THIS real write into a `TempDir` assets root and
/// loads it back through the REAL injuries folder walk, never polluting the
/// shipped `assets/` tree. The target subfolder derives from the def's own
/// authored `category` ([`injury_save_path_in`]).
///
/// Resolves the sanitized path and hands the serialize → mkdir → write chain to
/// the shared [`write_ron_pretty`] seam (GTW-577 C2). Returns the written path on
/// success so the caller can log it. Debug-only (the terrain / theme / gang /
/// armor save precedent): the fs write never compiles into a release binary.
///
/// # Errors
///
/// The seam's [`RonSaveError`], whose `Display` names the failed stage (serialize
/// vs write).
#[cfg(debug_assertions)]
pub fn write_injury_in(
    root: &Path,
    key: &InjuryName,
    def: &InjuryDef,
) -> Result<PathBuf, RonSaveError> {
    let path = injury_save_path_in(root, def.category, key);
    write_ron_pretty(&path, def)?;
    Ok(path)
}

/// Write an injury def to the workspace `assets/` tree — [`write_injury_in`] under
/// the shared [`WORKSPACE_ASSETS_ROOT`] owner (byte-identical to the app's
/// `AssetPlugin.file_path` by construction), so the saved def lands exactly where
/// the GTW-437 folder loader reads. The thin root-supplying wrapper the Save
/// button calls.
///
/// # Errors
///
/// The seam's [`RonSaveError`] (see [`write_injury_in`]).
#[cfg(debug_assertions)]
pub fn write_injury(key: &InjuryName, def: &InjuryDef) -> Result<PathBuf, RonSaveError> {
    write_injury_in(Path::new(WORKSPACE_ASSETS_ROOT), key, def)
}

/// Serialize + WRITE a weighting table under an arbitrary assets `root` — the
/// [`write_injury_in`] parity for the second artifact kind: the target file
/// derives from the record's own `category` ([`weighting_save_path_in`]), so a
/// saved table overwrites exactly the shipped per-category file the loader folds.
/// Debug-only (the same save-precedent gate).
///
/// # Errors
///
/// The seam's [`RonSaveError`] (see [`write_injury_in`]).
#[cfg(debug_assertions)]
pub fn write_weighting_in(
    root: &Path,
    weighting: &InjuryWeighting,
) -> Result<PathBuf, RonSaveError> {
    let path = weighting_save_path_in(root, weighting.category);
    write_ron_pretty(&path, weighting)?;
    Ok(path)
}

/// Write a weighting table to the workspace `assets/` tree —
/// [`write_weighting_in`] under the shared [`WORKSPACE_ASSETS_ROOT`] owner. The
/// thin root-supplying wrapper the weighting section's Save button calls.
///
/// # Errors
///
/// The seam's [`RonSaveError`] (see [`write_injury_in`]).
#[cfg(debug_assertions)]
pub fn write_weighting(weighting: &InjuryWeighting) -> Result<PathBuf, RonSaveError> {
    write_weighting_in(Path::new(WORKSPACE_ASSETS_ROOT), weighting)
}
