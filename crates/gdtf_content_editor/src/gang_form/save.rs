//! The GANG-mode form's **projection + path + write** (GTW-636 C1): turn the
//! [`GangDraft`] into the loader's `(`[`GangName`]`, `[`GangRoster`]`)` pair, resolve the
//! one-owner save location, and write it through the shared RON writer so the GTW-415
//! gangs folder loader reads back exactly what was saved.
//!
//! Every path segment is DERIVED, never re-spelled (GTW-621/634): the root is the shared
//! [`WORKSPACE_ASSETS_ROOT`] owner, the folder / compound extension come from
//! [`GangsFamily`]'s `FOLDER` / `EXTENSION`, and the stem runs through the shared
//! [`sanitize_file_stem`] helper. [`draft_to_roster`] / [`gang_file_name`] /
//! [`gang_save_path_in`] are PURE (no IO) so tests can round-trip them without touching
//! the assets tree; the filesystem write lives in [`write_gang_in`] (root-parameterized —
//! the GTW-555 `write_terrain_in` precedent, so tests aim it at a `TempDir`) and its thin
//! production wrapper [`write_gang`] (both debug-only, the terrain/theme save precedent).

use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_battle_sim::ganger::{GangName, GangRoster};
use gdtf_content_families::GangsFamily;

use super::draft::GangDraft;

/// Project the in-progress [`GangDraft`] into the loader's `(`[`GangName`]`,
/// `[`GangRoster`]`)` pair — the EXACT schema the GTW-415 gangs loader reads (the
/// GTW-429 round-trip contract), never a parallel one. The name buffer folds (trimmed)
/// into the registry-key [`GangName`]; the members are the sim records themselves, so
/// the roster is a member-list copy.
///
/// Pure (no IO) so the round-trip tests project through the SAME conversion the Save
/// button runs.
#[must_use]
pub fn draft_to_roster(draft: &GangDraft) -> (GangName, GangRoster) {
    let name = GangName::new(draft.name().trim().to_owned());
    let roster = GangRoster::new(draft.members().iter().cloned());
    (name, roster)
}

/// The on-disk FILE NAME for a saved gang — `<sanitized_gang_name>.gang.ron`.
///
/// The suffix is DERIVED from [`GangsFamily::EXTENSION`] (the ONE canonical extension
/// discriminant the gangs folder loader dispatches on — GTW-621: the pre-derivation
/// `<stem>.ron` write drifted and every saved gang silently vanished on reload). The stem
/// runs through the shared [`sanitize_file_stem`] helper (GTW-577) so a path-hostile gang
/// name can never reach the filesystem raw; a name that sanitizes to NOTHING falls back
/// to the documented `unnamed_gang` stem (minted through the SAME helper — the retired
/// in-game editor's convention, kept for parity). The loader keys a gang by its file stem
/// with the `.gang` infix stripped, so a saved gang reloads keyed by exactly its
/// sanitized stem.
///
/// Pure (no IO) so a test can assert the resolved name without writing anything.
#[must_use]
pub fn gang_file_name(name: &GangName) -> String {
    let stem = sanitize_file_stem(name.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed gang")
    } else {
        stem
    };
    format!("{stem}.{}", GangsFamily::EXTENSION)
}

/// The full on-disk PATH a saved gang is written to under an arbitrary assets `root`:
/// `<root>/`[`GangsFamily::FOLDER`]`/` joined with the [`gang_file_name`] — the
/// root-parameterized core (the GTW-555 pattern), so a test resolves the REAL save
/// location against a `TempDir` root instead of the version-controlled `assets/` tree.
///
/// Pure (no IO) so a test can assert the resolved location without writing anything.
#[must_use]
pub fn gang_save_path_in(root: &Path, name: &GangName) -> PathBuf {
    root.join(GangsFamily::FOLDER).join(gang_file_name(name))
}

/// Serialize + WRITE a gang roster under an arbitrary assets `root` — the
/// root-parameterized write core (the GTW-555 `write_terrain_in` precedent): the GTW-636
/// C5 round-trip test drives THIS real write into a `TempDir` assets root and loads it
/// back through the REAL gangs folder walk, never polluting the shipped `assets/` tree.
///
/// Resolves the sanitized path ([`gang_save_path_in`]) and hands the serialize → mkdir →
/// write chain to the shared [`write_ron_pretty`] writer (GTW-577 C2). Returns the written
/// path on success so the caller can log it. Debug-only (the terrain / theme save
/// precedent): the fs write never compiles into a release binary.
///
/// # Errors
///
/// The writer's [`RonSaveError`], whose `Display` names the failed stage (serialize vs
/// write).
#[cfg(debug_assertions)]
pub fn write_gang_in(
    root: &Path,
    name: &GangName,
    roster: &GangRoster,
) -> Result<PathBuf, RonSaveError> {
    let path = gang_save_path_in(root, name);
    write_ron_pretty(&path, roster)?;
    Ok(path)
}

/// Write a gang roster to the workspace `assets/` tree — [`write_gang_in`] under the
/// shared [`WORKSPACE_ASSETS_ROOT`] owner (identical to the app's
/// `AssetPlugin.file_path` by construction), so the saved gang lands exactly where the
/// GTW-415 folder loader reads. The thin root-supplying wrapper the Save button calls.
///
/// # Errors
///
/// The writer's [`RonSaveError`] (see [`write_gang_in`]).
#[cfg(debug_assertions)]
pub fn write_gang(name: &GangName, roster: &GangRoster) -> Result<PathBuf, RonSaveError> {
    write_gang_in(Path::new(WORKSPACE_ASSETS_ROOT), name, roster)
}
