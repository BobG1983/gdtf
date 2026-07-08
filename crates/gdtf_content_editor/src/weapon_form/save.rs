//! The WEAPON-mode form's **projection + path + write** (GTW-670 C3): turn the
//! [`WeaponDraft`] into the loader's `(`[`WeaponName`]`, `[`WeaponSpec`]`)` pair,
//! resolve the one-owner save location, and write it through the shared RON writer so
//! the GTW-257/570 [`WeaponsFamily`] folder loader reads back exactly what was saved.
//!
//! Every path segment is DERIVED, never re-spelled (GTW-621/634): the root is the shared
//! [`WORKSPACE_ASSETS_ROOT`] owner, the folder / compound extension come from
//! [`WeaponsFamily`]'s `FOLDER` / `EXTENSION`, and the stem runs through the shared
//! [`sanitize_file_stem`] seam. [`draft_to_weapon_spec`] / [`weapon_file_name`] /
//! [`weapon_save_path_in`] are PURE (no IO) so tests can round-trip them without
//! touching the assets tree; the filesystem write lives in [`write_weapon_in`]
//! (root-parameterized — the GTW-555 `write_terrain_in` precedent, so tests aim it at a
//! `TempDir`) and its thin production wrapper [`write_weapon`] (both debug-only, the
//! gang / armor / sprite / attachment save precedent).
//!
//! RE-VALIDATION IS FREE (GTW-670 C4, cited not rebuilt): a save overwrites the member
//! on disk, the family redrive rebuilds the
//! [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry) from it, and the
//! editor's validation re-arm watch set (`validate/rearm.rs`,
//! `WatchedRegistries::weapons` — watched since GTW-630 for the emplacement + gang
//! edges) watches that registry — so the weapon→attachment integrity edge (GTW-669)
//! and every other registered check re-run live on every weapon save with zero new
//! machinery here.

use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_battle_sim::weapon::{WeaponName, WeaponSpec};
use gdtf_content_families::WeaponsFamily;

use super::draft::WeaponDraft;

/// Project the in-progress [`WeaponDraft`] into the loader's `(`[`WeaponName`]`,
/// `[`WeaponSpec`]`)` pair — the EXACT schema the GTW-257 ranged-weapons loader reads
/// (the GTW-636 round-trip contract), never a parallel one. The name buffer folds
/// (trimmed) into the registry-key [`WeaponName`]; the spec is the sim record itself,
/// so the projection is a copy.
///
/// Pure (no IO) so the round-trip tests project through the SAME conversion the Save
/// button runs.
#[must_use]
pub fn draft_to_weapon_spec(draft: &WeaponDraft) -> (WeaponName, WeaponSpec) {
    let name = WeaponName::new(draft.name().trim().to_owned());
    (name, draft.spec().clone())
}

/// The on-disk FILE NAME for a saved weapon — `<sanitized_name>.weapon.ron`.
///
/// The suffix is DERIVED from [`WeaponsFamily::EXTENSION`] (the ONE canonical extension
/// discriminant the ranged-weapons folder loader dispatches on — GTW-621: a re-spelled
/// extension drifts and every saved file silently vanishes on reload). The stem runs
/// through the shared [`sanitize_file_stem`] seam (GTW-577) so a path-hostile weapon
/// name can never reach the filesystem raw; a name that sanitizes to NOTHING falls back
/// to the documented `unnamed_weapon` stem (minted through the SAME seam — the gang /
/// armor / sprite / attachment save parity). The loader keys a weapon by its file stem,
/// so a saved weapon reloads keyed by exactly its sanitized stem.
///
/// Pure (no IO) so a test can assert the resolved name without writing anything.
#[must_use]
pub fn weapon_file_name(name: &WeaponName) -> String {
    let stem = sanitize_file_stem(name.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed weapon")
    } else {
        stem
    };
    format!("{stem}.{}", WeaponsFamily::EXTENSION)
}

/// The full on-disk PATH a saved weapon is written to under an arbitrary assets
/// `root`: `<root>/`[`WeaponsFamily::FOLDER`]`/` joined with the [`weapon_file_name`]
/// — the root-parameterized core (the GTW-555 pattern), so a test resolves the REAL
/// save location against a `TempDir` root instead of the version-controlled `assets/`
/// tree.
///
/// Pure (no IO) so a test can assert the resolved location without writing anything.
#[must_use]
pub fn weapon_save_path_in(root: &Path, name: &WeaponName) -> PathBuf {
    root.join(WeaponsFamily::FOLDER)
        .join(weapon_file_name(name))
}

/// Serialize + WRITE a weapon under an arbitrary assets `root` — the root-parameterized
/// write core (the GTW-555 `write_terrain_in` precedent): the GTW-670 round-trip test
/// drives THIS real write into a `TempDir` assets root and loads it back through the
/// REAL ranged-weapons folder walk, never polluting the shipped `assets/` tree.
///
/// Resolves the sanitized path ([`weapon_save_path_in`]) and hands the serialize →
/// mkdir → write chain to the shared [`write_ron_pretty`] seam (GTW-577 C2). Returns
/// the written path on success so the caller can log it. Debug-only (the terrain /
/// theme / gang / armor / sprite / attachment save precedent): the fs write never
/// compiles into a release binary.
///
/// # Errors
///
/// The seam's [`RonSaveError`], whose `Display` names the failed stage (serialize vs
/// write).
#[cfg(debug_assertions)]
pub fn write_weapon_in(
    root: &Path,
    name: &WeaponName,
    spec: &WeaponSpec,
) -> Result<PathBuf, RonSaveError> {
    let path = weapon_save_path_in(root, name);
    write_ron_pretty(&path, spec)?;
    Ok(path)
}

/// Write a weapon to the workspace `assets/` tree — [`write_weapon_in`] under the
/// shared [`WORKSPACE_ASSETS_ROOT`] owner (byte-identical to the app's
/// `AssetPlugin.file_path` by construction), so the saved weapon lands exactly where
/// the GTW-257 folder loader reads. The thin root-supplying wrapper the Save button
/// calls.
///
/// # Errors
///
/// The seam's [`RonSaveError`] (see [`write_weapon_in`]).
#[cfg(debug_assertions)]
pub fn write_weapon(name: &WeaponName, spec: &WeaponSpec) -> Result<PathBuf, RonSaveError> {
    write_weapon_in(Path::new(WORKSPACE_ASSETS_ROOT), name, spec)
}
