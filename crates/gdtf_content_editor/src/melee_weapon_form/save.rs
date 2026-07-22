//! The MELEE-WEAPON-mode form's **projection + path + write** (GTW-671 C3): turn the
//! [`MeleeWeaponDraft`] into the loader's `(`[`WeaponName`]`, `[`MeleeWeaponSpec`]`)`
//! pair, resolve the one-owner save location, and write it through the shared RON
//! writer so the GTW-505/570 [`MeleeWeaponsFamily`] folder loader reads back exactly
//! what was saved.
//!
//! Every path segment is DERIVED, never re-spelled (GTW-621/634): the root is the
//! shared [`WORKSPACE_ASSETS_ROOT`] owner, the folder / compound extension come from
//! [`MeleeWeaponsFamily`]'s `FOLDER` / `EXTENSION`, and the stem runs through the
//! shared [`sanitize_file_stem`] helper. [`draft_to_melee_weapon_spec`] /
//! [`melee_weapon_file_name`] / [`melee_weapon_save_path_in`] are PURE (no IO) so tests
//! can round-trip them without touching the assets tree; the filesystem write lives in
//! [`write_melee_weapon_in`] (root-parameterized — the GTW-555 `write_terrain_in`
//! precedent, so tests aim it at a `TempDir`) and its thin production wrapper
//! [`write_melee_weapon`] (both debug-only, the gang / armor / sprite / attachment /
//! weapon save precedent).
//!
//! RE-VALIDATION IS FREE (GTW-671 C4, cited not rebuilt): a save overwrites the member
//! on disk, the family redrive rebuilds the
//! [`MeleeWeaponRegistry`](gdtf_battle_sim::weapon::MeleeWeaponRegistry) from it, and
//! the editor's validation re-arm watch set (`validate/rearm.rs`,
//! `WatchedRegistries::melee_weapons` — watched since GTW-651 for the gang equipment
//! edge) watches that registry — so the gang→melee-weapon key edge (GTW-651) and the
//! melee side of the weapon→attachment edge (`check_weapon_attachment_refs` chains the
//! melee registry since GTW-669) re-run live on every melee-weapon save with zero new
//! machinery here.

use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_battle_sim::weapon::{MeleeWeaponSpec, WeaponName};
use gdtf_content_families::MeleeWeaponsFamily;

use super::draft::MeleeWeaponDraft;

/// Project the in-progress [`MeleeWeaponDraft`] into the loader's `(`[`WeaponName`]`,
/// `[`MeleeWeaponSpec`]`)` pair — the EXACT schema the GTW-505 melee-weapons loader
/// reads (the GTW-636 round-trip contract), never a parallel one. The name buffer folds
/// (trimmed) into the registry-key [`WeaponName`]; the spec is the sim record itself,
/// so the projection is a copy.
///
/// Pure (no IO) so the round-trip tests project through the SAME conversion the Save
/// button runs.
#[must_use]
pub fn draft_to_melee_weapon_spec(draft: &MeleeWeaponDraft) -> (WeaponName, MeleeWeaponSpec) {
    let name = WeaponName::new(draft.name().trim().to_owned());
    (name, draft.spec().clone())
}

/// The on-disk FILE NAME for a saved melee weapon — `<sanitized_name>.melee_weapon.ron`.
///
/// The suffix is DERIVED from [`MeleeWeaponsFamily::EXTENSION`] (the ONE canonical
/// extension discriminant the melee-weapons folder loader dispatches on — GTW-621: a
/// re-spelled extension drifts and every saved file silently vanishes on reload). The
/// stem runs through the shared [`sanitize_file_stem`] helper (GTW-577) so a path-hostile
/// weapon name can never reach the filesystem raw; a name that sanitizes to NOTHING
/// falls back to the documented `unnamed_melee_weapon` stem (minted through the SAME
/// helper — the gang / armor / sprite / attachment / weapon save parity). The loader keys
/// a melee weapon by its file stem, so a saved weapon reloads keyed by exactly its
/// sanitized stem.
///
/// Pure (no IO) so a test can assert the resolved name without writing anything.
#[must_use]
pub fn melee_weapon_file_name(name: &WeaponName) -> String {
    let stem = sanitize_file_stem(name.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed melee weapon")
    } else {
        stem
    };
    format!("{stem}.{}", MeleeWeaponsFamily::EXTENSION)
}

/// The full on-disk PATH a saved melee weapon is written to under an arbitrary assets
/// `root`: `<root>/`[`MeleeWeaponsFamily::FOLDER`]`/` joined with the
/// [`melee_weapon_file_name`] — the root-parameterized core (the GTW-555 pattern), so a
/// test resolves the REAL save location against a `TempDir` root instead of the
/// version-controlled `assets/` tree.
///
/// Pure (no IO) so a test can assert the resolved location without writing anything.
#[must_use]
pub fn melee_weapon_save_path_in(root: &Path, name: &WeaponName) -> PathBuf {
    root.join(MeleeWeaponsFamily::FOLDER)
        .join(melee_weapon_file_name(name))
}

/// Serialize + WRITE a melee weapon under an arbitrary assets `root` — the
/// root-parameterized write core (the GTW-555 `write_terrain_in` precedent): the
/// GTW-671 round-trip test drives THIS real write into a `TempDir` assets root and
/// loads it back through the REAL melee-weapons folder walk, never polluting the
/// shipped `assets/` tree.
///
/// Resolves the sanitized path ([`melee_weapon_save_path_in`]) and hands the serialize
/// → mkdir → write chain to the shared [`write_ron_pretty`] writer (GTW-577 C2). Returns
/// the written path on success so the caller can log it. Debug-only (the terrain /
/// theme / gang / armor / sprite / attachment / weapon save precedent): the fs write
/// never compiles into a release binary.
///
/// # Errors
///
/// The writer's [`RonSaveError`], whose `Display` names the failed stage (serialize vs
/// write).
#[cfg(debug_assertions)]
pub fn write_melee_weapon_in(
    root: &Path,
    name: &WeaponName,
    spec: &MeleeWeaponSpec,
) -> Result<PathBuf, RonSaveError> {
    let path = melee_weapon_save_path_in(root, name);
    write_ron_pretty(&path, spec)?;
    Ok(path)
}

/// Write a melee weapon to the workspace `assets/` tree — [`write_melee_weapon_in`]
/// under the shared [`WORKSPACE_ASSETS_ROOT`] owner (identical to the app's
/// `AssetPlugin.file_path` by construction), so the saved weapon lands exactly where
/// the GTW-505 folder loader reads. The thin root-supplying wrapper the Save button
/// calls.
///
/// # Errors
///
/// The writer's [`RonSaveError`] (see [`write_melee_weapon_in`]).
#[cfg(debug_assertions)]
pub fn write_melee_weapon(
    name: &WeaponName,
    spec: &MeleeWeaponSpec,
) -> Result<PathBuf, RonSaveError> {
    write_melee_weapon_in(Path::new(WORKSPACE_ASSETS_ROOT), name, spec)
}
