//! The gang SAVE system: a press on the "Save gang" button serializes the edited
//! [`EditableGang`](super::super::model::EditableGang) into the GTW-415
//! [`GangRoster`](gdtf_battle_sim::GangRoster) schema and WRITES it to
//! `assets/content/gangs/<sanitized_gang_name>.gang.ron` (GTW-429; the stem is sanitized
//! through the shared GTW-577 [`sanitize_file_stem`](gdtf_assets::sanitize_file_stem) seam;
//! the extension is DERIVED from `GangsFamily::EXTENSION` since GTW-621 so the write can
//! never drift from the gangs folder loader's read).
//!
//! The save action's live-play trigger (C4): the editor toolbar's [`SaveGangButton`]. A press
//! projects the model to its sim `(`[`GangName`](gdtf_battle_sim::GangName)`,
//! `[`GangRoster`](gdtf_battle_sim::GangRoster)`)` via
//! [`to_roster`](super::super::model::EditableGang::to_roster) — the SAME def the loader reads,
//! NOT a parallel schema (C1) — serializes the roster to RON, and writes it under the workspace
//! assets root the running app loads from, keyed by the gang NAME (the user 2026-07-04 ruling,
//! revising the 2026-06-26 `<stem>.ron` one after GTW-621 showed a bare `.ron` never reloads).
//!
//! The whole module is `#[cfg(debug_assertions)]`-gated (C3): the fs-write fn + the press system
//! are NOT compiled into a release binary. The editor scene itself is debug-only (its menu entry
//! is `cfg(debug_assertions)`), but the filesystem write is gated EXPLICITLY here so a release
//! build can never reach the disk-write code path.
//!
//! The write handles its `io` / serialization [`Result`](std::result::Result) by LOGGING an
//! `error!` on failure (a missing dir, a permissions error, a serialization error) — it never
//! `unwrap`/`expect`/`panic`s (the workspace-denied lints + the no-panic-in-the-happy-path rule).

use std::path::{Path, PathBuf};

use bevy::{prelude::*, ui::Interaction};
#[cfg(test)]
use gdtf_assets::serialize_ron_pretty;
use gdtf_assets::{ContentFamily, WORKSPACE_ASSETS_ROOT, sanitize_file_stem, write_ron_pretty};
use gdtf_battle_sim::{GangName, GangRoster};
use gdtf_content_families::GangsFamily;

use crate::states::running::gang_editor::{components::SaveGangButton, model::EditableGang};

// GTW-634 C1/C3: the assets root and the gangs folder are NOT re-spelled here — the root is
// the shared [`WORKSPACE_ASSETS_ROOT`] owner (byte-identical to the app's
// `AssetPlugin.file_path` by construction) and the folder is DERIVED from
// [`GangsFamily::FOLDER`], joining the extension GTW-621 already derived from
// [`GangsFamily::EXTENSION`] — so the saved gang lands exactly where the GTW-415 folder
// loader reads, with zero hand-maintained mirrors.

/// The on-disk FILE NAME for a saved gang — `<sanitized_gang_name>.gang.ron` (the user
/// 2026-07-04 ruling, revising the 2026-06-26 `<stem>.ron` one; sanitized through the shared
/// seam since GTW-577 C1), a pure function of the [`GangName`] so a test can assert it WITHOUT
/// touching the filesystem.
///
/// The suffix is DERIVED from [`GangsFamily::EXTENSION`] — the ONE canonical extension
/// discriminant, the exact compound extension the gangs folder loader is registered for — so
/// the editor's WRITE can never drift from the loader's READ again (GTW-621 C3). The
/// pre-GTW-621 `<stem>.ron` write drifted: the bare `.ron` dispatched to another plain-`ron`
/// loader, the gangs family's `TypeId` filter skipped the member, and every saved gang
/// silently vanished on reload.
///
/// The stem is built through [`sanitize_file_stem`], so a path-hostile gang name (`"../../X"`,
/// spaces, uppercase) can never reach the filesystem raw — before GTW-577 the raw name went
/// straight into the file name. A name that sanitizes to NOTHING falls back to the documented
/// `unnamed_gang` stem (minted through the SAME seam), so a save never targets a bare,
/// extension-only file name.
///
/// NOTE: the GTW-415 loader keys a gang by its file STEM with the `.gang` infix stripped
/// (`gang_0.gang.ron` keys `gang_0`), so a saved gang reloads keyed by exactly its sanitized
/// stem (the loader's own slug convention).
#[must_use]
pub(in crate::states::running::gang_editor) fn gang_file_name(name: &GangName) -> String {
    let stem = sanitize_file_stem(name.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed gang")
    } else {
        stem
    };
    format!("{stem}.{}", GangsFamily::EXTENSION)
}

/// The full on-disk PATH a saved gang is written to under an arbitrary assets `root`:
/// `<root>/content/gangs/` joined with the [`gang_file_name`] — the root-parameterized core
/// (the GTW-555 `write_terrain_in` precedent), so a test can resolve the REAL save location
/// against a `TempDir` root instead of the version-controlled `assets/` tree.
///
/// Pure (no IO) so a test can assert the resolved location without writing anything.
#[must_use]
pub(in crate::states::running::gang_editor) fn gang_save_path_in(
    root: &Path,
    name: &GangName,
) -> PathBuf {
    root.join(GangsFamily::FOLDER).join(gang_file_name(name))
}

/// The full on-disk PATH the RUNNING app saves a gang to:
/// [`gang_save_path_in`] under the workspace [`WORKSPACE_ASSETS_ROOT`].
///
/// Test-only since GTW-621: the production write chain resolves its path inside
/// [`write_gang_roster_in`] (via the root the wrapper supplies), so this thin delegation
/// exists for the path-sanitization test to assert the resolved workspace location.
#[cfg(test)]
#[must_use]
fn gang_save_path(name: &GangName) -> PathBuf {
    gang_save_path_in(Path::new(WORKSPACE_ASSETS_ROOT), name)
}

/// Serialize the gang roster to its `*.gang.ron`-shaped RON text — the SAME schema the GTW-415
/// loader deserializes (C1), through the SAME shared [`serialize_ron_pretty`] seam the
/// production write ([`write_ron_pretty`] inside [`write_gang_roster`]) serializes with
/// (GTW-577 C2).
///
/// Test-only since GTW-577: the production chain runs entirely inside the seam, so this thin
/// delegation exists for the round-trip test to produce the exact bytes the save would write
/// WITHOUT touching the filesystem (C2).
#[cfg(test)]
fn serialize_roster(roster: &GangRoster) -> Result<String, gdtf_assets::RonSaveError> {
    serialize_ron_pretty(roster)
}

crate::support_item! {
    /// Write the edited gang under an arbitrary assets `root` — the root-parameterized write core
    /// (the GTW-555 `write_terrain_in` precedent), so the GTW-621 regression test can drive the
    /// REAL write into a `TempDir` assets root and then load it back through the REAL gangs
    /// folder walk, never polluting the version-controlled `assets/` tree.
    ///
    /// Resolves the sanitized save path ([`gang_save_path_in`], which builds its stem through the
    /// shared [`sanitize_file_stem`] seam) and hands the serialize → mkdir → write chain to the
    /// shared [`write_ron_pretty`] seam (GTW-577 C2). The fallible write is handled by LOGGING an
    /// `error!` — it NEVER `unwrap`/`expect`/`panic`s (the no-panic rule); the seam error's
    /// `Display` names the failed stage (serialize vs write). A success logs an `info!` naming
    /// the written path.
    ///
    /// The whole module is `#[cfg(debug_assertions)]`-gated (C3), so this filesystem write is
    /// never compiled into a release binary. Declared through `crate::support_item!` (`pub`
    /// under `test-support`, `pub(crate)` otherwise) so the GTW-621 regression test can reach
    /// it through the crate-root ledger.
    fn write_gang_roster_in(root: &Path, name: &GangName, roster: &GangRoster) {
        let path = gang_save_path_in(root, name);
        if let Err(err) = write_ron_pretty(&path, roster) {
            error!("gang save: gang `{}`: {err}", name.as_str());
            return;
        }
        info!(
            "gang save: wrote gang `{}` to `{}`",
            name.as_str(),
            path.display()
        );
    }
}

/// Write the edited gang to `assets/content/gangs/` (C1): [`write_gang_roster_in`] under the
/// workspace [`WORKSPACE_ASSETS_ROOT`] — the thin root-supplying wrapper the Save-button
/// system calls.
fn write_gang_roster(name: &GangName, roster: &GangRoster) {
    write_gang_roster_in(Path::new(WORKSPACE_ASSETS_ROOT), name, roster);
}

/// `Update` (gated `in_state(DebugGangEditor)`): writes the edited gang to disk on a "Save gang" press
/// (C4).
///
/// Reads the [`SaveGangButton`]'s [`Interaction`] (the menu-action / add-member press precedent) and
/// fires once per press edge. It projects the [`EditableGang`] model to its sim `(`[`GangName`]`,
/// `[`GangRoster`]`)` pair — the SAME schema the loader reads (C1) — and writes it via
/// [`write_gang_roster`]. Guarded on the model's presence via `Option<ResMut<…>>` (the model is a
/// state-scoped resource — `bevy-traps.md` #1). The whole module is `#[cfg(debug_assertions)]`-gated
/// (C3), so this disk-write system is never compiled into a release binary.
///
/// Param-only (`bevy-traps.md` #7): the [`Query`] + the optional [`EditableGang`] borrow.
pub(in crate::states::running::gang_editor) fn save_gang_on_press(
    buttons: Query<&Interaction, (Changed<Interaction>, With<SaveGangButton>)>,
    model: Option<ResMut<EditableGang>>,
) {
    let pressed = buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    if !pressed {
        return;
    }
    let Some(model) = model else {
        return;
    };
    let (name, roster) = model.to_roster();
    write_gang_roster(&name, &roster);
}

#[cfg(test)]
mod tests {
    use gdtf_assets::ContentFamily;
    use gdtf_battle_sim::{
        Aim, ArmorName, Cool, GangMember, GangName, GangRegistry, GangRoster, GangerName, Grit,
        Reflexes, Speed, Strength, Toughness, WeaponName, ganger::Luck,
    };
    use gdtf_content_families::GangsFamily;

    use super::{gang_file_name, gang_save_path, serialize_roster};
    use crate::states::running::gang_editor::model::EditableGang;

    /// A two-member edited gang's REGISTRY fixture — built from the fully-public sim
    /// [`GangRoster`] / [`GangMember`] types so the test does not reach the editor model's private
    /// fields. The magnitudes are arbitrary fixture data (NOT pinned shipped tuning): the test
    /// asserts the values SURVIVE the round-trip, never that they equal a shipped number.
    fn edited_registry() -> GangRegistry {
        let roster = GangRoster::new([
            GangMember {
                name:         GangerName::new("Edited Alpha".to_owned()),
                speed:        Speed::new(2.5),
                aim:          Aim::new(3.0),
                strength:     Strength::new(4.0),
                toughness:    Toughness::new(11.0),
                reflexes:     Reflexes::new(3.5),
                cool:         Cool::new(6.0),
                grit:         Grit::new(18.0),
                luck:         Luck::new(1.0),
                armor:        ArmorName::new("flak_vest".to_owned()),
                weapon:       WeaponName::new("stub_pistol".to_owned()),
                melee_weapon: None,
            },
            GangMember {
                name:         GangerName::new("Edited Bravo".to_owned()),
                speed:        Speed::new(4.0),
                aim:          Aim::new(5.0),
                strength:     Strength::new(2.0),
                toughness:    Toughness::new(9.0),
                reflexes:     Reflexes::new(4.0),
                cool:         Cool::new(7.0),
                grit:         Grit::new(15.0),
                luck:         Luck::new(2.0),
                armor:        ArmorName::new("mesh_armor".to_owned()),
                weapon:       WeaponName::new("autogun".to_owned()),
                melee_weapon: None,
            },
        ]);
        GangRegistry::new([(GangName::new("edited_gang".to_owned()), roster)])
    }

    /// GTW-429 C2 — the IDENTITY round-trip: an edited [`EditableGang`] → serialize into the
    /// GTW-415 [`GangRoster`] schema → reload through the SAME loader path (deserialize +
    /// key-by-file-stem into a [`GangRegistry`]) → the reloaded model EQUALS the edited one.
    ///
    /// The reload mirrors the GTW-415 loader EXACTLY: it deserializes the serialized bytes with
    /// `ron::de::from_str::<GangRoster>` — the parser `RonAsset<GangRoster>` uses — and keys the
    /// roster into a [`GangRegistry`] by the saved file's STEM with a trailing `.gang` stripped
    /// (the loader's stem keying). The editor saves `<name>.gang.ron` (GTW-621), so the strip
    /// recovers exactly the gang name.
    ///
    /// In-memory string round-trip — it NEVER writes into the real `assets/content/gangs/` dir (no
    /// repo pollution, deterministic). Structural equality across the WHOLE model (gang name + each
    /// member name + the eight attributes + weapon / armor keys) via the model's derived
    /// [`PartialEq`], with NO pinned magnitudes — identity, not a number lock.
    ///
    /// Pin-discriminating: dropping any field from `to_sim`/`to_roster` (or mis-keying the reload)
    /// breaks the final equality.
    #[test]
    fn edited_gang_round_trips_through_the_loader() {
        // The edited working model — seeded from a registry exactly as the editor opens it.
        let edited = EditableGang::from_registry(&edited_registry());

        // SAVE: project to the sim `(GangName, GangRoster)` and serialize into the GTW-415 schema.
        let (name, roster) = edited.to_roster();
        let serialized = serialize_roster(&roster);
        assert!(
            serialized.is_ok(),
            "serializing the edited roster must succeed: {:?}",
            serialized.as_ref().err(),
        );
        let Ok(serialized) = serialized else {
            return;
        };
        // The on-disk file name the save path would use — `<gang_name>.gang.ron`.
        let file_name = gang_file_name(&name);

        // RELOAD via the loader path: deserialize the bytes the way `RonAsset<GangRoster>` does …
        let reloaded_roster = ron::de::from_str::<GangRoster>(&serialized);
        assert!(
            reloaded_roster.is_ok(),
            "the serialized gang must round-trip through the GangRoster deserializer: {:?}",
            reloaded_roster.as_ref().err(),
        );
        let Ok(reloaded_roster) = reloaded_roster else {
            return;
        };
        // … and key it into a registry by the file STEM minus the `.gang` infix (the family
        // seam's `stem_from_path` keying), mirroring the folder-walk registry build.
        let stem = file_name
            .strip_suffix(".ron")
            .map_or_else(|| file_name.clone(), ToOwned::to_owned);
        let key = stem.strip_suffix(".gang").unwrap_or(&stem).to_owned();
        let reloaded_registry = GangRegistry::new([(GangName::new(key), reloaded_roster)]);

        // Reopen the editor model from the reloaded registry and assert structural identity.
        let reloaded = EditableGang::from_registry(&reloaded_registry);
        assert_eq!(
            reloaded, edited,
            "the reloaded gang must equal the edited model (name + members + attributes + \
             weapon/armor keys) — the GTW-415 round-trip",
        );
    }

    /// The save FILE NAME is `<gang_name>.gang.ron` (the user 2026-07-04 ruling, revising the
    /// 2026-06-26 `<stem>.ron` one) — the loader's stem-keying strips the `.gang` infix and
    /// recovers exactly the gang name. An already-slug-shaped name passes through the GTW-577
    /// sanitize seam unchanged.
    #[test]
    fn save_file_name_is_gang_name_dot_gang_dot_ron() {
        let name = GangName::new("goliaths".to_owned());
        assert_eq!(gang_file_name(&name), "goliaths.gang.ron");
    }

    /// GTW-621 C4 — the written file name derives its suffix from the ONE canonical extension
    /// discriminant, [`GangsFamily::EXTENSION`] (the compound extension the gangs folder
    /// loader is registered for), so the editor's WRITE can never again drift from the
    /// loader's READ. The pre-GTW-621 `<stem>.ron` drift silently dropped every saved gang on
    /// reload: the bare `.ron` dispatched to another plain-`ron` loader and the gangs family's
    /// `TypeId` filter skipped it, with zero warns.
    #[test]
    fn save_file_name_derives_its_suffix_from_the_gangs_family_extension() {
        let file_name = gang_file_name(&GangName::new("goliaths".to_owned()));
        let expected_suffix = format!(".{}", GangsFamily::EXTENSION);
        assert!(
            file_name.ends_with(&expected_suffix),
            "the saved gang file name `{file_name}` must end with `{expected_suffix}` \
             (derived from GangsFamily::EXTENSION), or the gangs folder loader never \
             dispatches the saved file and the gang silently vanishes on reload",
        );
    }

    /// GTW-577 acceptance (2) — a PATH-HOSTILE [`GangName`] saves to a SANITIZED filename via
    /// the REAL [`gang_file_name`] / [`gang_save_path`]: the traversal-shaped `../../` and the
    /// `!` are dropped by the shared seam, spaces/uppercase fold to the slug convention, and
    /// the resolved path stays INSIDE `content/gangs/` (no `..` component survives). Before
    /// GTW-577 the raw name went straight into the filename.
    #[test]
    fn path_hostile_gang_name_saves_to_a_sanitized_filename() {
        let hostile = GangName::new("../../Evil Gang!".to_owned());
        assert_eq!(
            gang_file_name(&hostile),
            "evil_gang.gang.ron",
            "the seam drops path separators/dots and folds the name to the slug convention",
        );

        // The resolved path's TAIL is exactly the gangs family FOLDER + `<sanitized>.gang.ron`
        // — the hostile name contributed a single, separator-free FILE component, so it cannot
        // have escaped the gangs directory. The folder expectation is DERIVED from
        // `GangsFamily::FOLDER` (GTW-634 A1: the folder literal has ONE owning spelling — the
        // family impl the loader walks), so this pins the name-derived tail against the exact
        // folder the loader reads.
        let path = gang_save_path(&hostile);
        let expected_tail = std::path::Path::new(GangsFamily::FOLDER).join("evil_gang.gang.ron");
        assert!(
            path.ends_with(&expected_tail),
            "the sanitized file lands under the gangs family folder: {path:?}",
        );
        assert_eq!(
            path.file_name().and_then(|f| f.to_str()),
            Some("evil_gang.gang.ron"),
            "the whole hostile name collapsed into ONE sanitized file component",
        );

        // A name that sanitizes to NOTHING falls back to the documented stem — never a bare,
        // extension-only file name.
        let unnameable = GangName::new("!!!///".to_owned());
        assert_eq!(gang_file_name(&unnameable), "unnamed_gang.gang.ron");
    }
}
