//! The gang SAVE system: a press on the "Save gang" button serializes the edited
//! [`EditableGang`](super::super::model::EditableGang) into the GTW-415
//! [`GangRoster`](gdtf_battle_sim::GangRoster) schema and WRITES it to
//! `assets/content/gangs/<gang_name>.ron` (GTW-429).
//!
//! The save action's live-play trigger (C4): the editor toolbar's [`SaveGangButton`]. A press
//! projects the model to its sim `(`[`GangName`](gdtf_battle_sim::GangName)`,
//! `[`GangRoster`](gdtf_battle_sim::GangRoster)`)` via
//! [`to_roster`](super::super::model::EditableGang::to_roster) — the SAME def the loader reads,
//! NOT a parallel schema (C1) — serializes the roster to RON, and writes it under the workspace
//! assets root the running app loads from, keyed by the gang NAME (the RESOLVED save path, user
//! 2026-06-26).
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
use gdtf_battle_sim::{GangName, GangRoster};

use crate::states::running::editor::{components::SaveGangButton, model::EditableGang};

/// The workspace `assets/` root — byte-identical to the app's `AssetPlugin.file_path`
/// (`crates/gdtf_app` → up two levels → `assets`), computed at compile time relative to THIS
/// crate's manifest. So a gang the editor SAVES lands exactly where the running app (and the
/// GTW-415 folder loader) READS gangs from — `assets/content/gangs/`.
const WORKSPACE_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

/// The folder under the assets root the GTW-415 loader scans for `*.gang.ron` gang files — the
/// directory the saved gang `.ron` is written into.
const GANGS_SUBDIR: &str = "content/gangs";

/// The on-disk FILE NAME for a saved gang — `<gang_name>.ron` (the RESOLVED save path, user
/// 2026-06-26), a pure function of the [`GangName`] so a test can assert it WITHOUT touching the
/// filesystem.
///
/// NOTE: the GTW-415 loader keys a gang by its file STEM with a trailing `.gang` infix stripped
/// (`gang_0.gang.ron` keys `gang_0`), so a plain `<name>.ron` file keys back to exactly `<name>`
/// — the editor's save target deliberately omits the `.gang` infix, and the loader's
/// `strip_suffix(".gang")` is a no-op on it, so the round-trip key is the gang name unchanged.
#[must_use]
pub(in crate::states::running::editor) fn gang_file_name(name: &GangName) -> String {
    format!("{}.ron", name.as_str())
}

/// The full on-disk PATH a saved gang is written to:
/// `<workspace assets>/content/gangs/<gang_name>.ron`.
///
/// Pure (no IO) so a test can assert the resolved location ends in the expected
/// `content/gangs/<name>.ron` without writing anything.
#[must_use]
pub(in crate::states::running::editor) fn gang_save_path(name: &GangName) -> PathBuf {
    Path::new(WORKSPACE_ASSETS_ROOT)
        .join(GANGS_SUBDIR)
        .join(gang_file_name(name))
}

/// Serialize the gang roster to its `*.gang.ron`-shaped RON text — the SAME schema the GTW-415
/// loader deserializes (C1). A pretty-printed record so a saved gang stays human-editable like
/// the shipped `assets/content/gangs/*.gang.ron`.
///
/// Returns the serialization [`Result`](std::result::Result) so the caller logs (never panics) on
/// a failure; the round-trip test reuses it to produce the bytes it reloads through the loader (C2).
pub(in crate::states::running::editor) fn serialize_roster(
    roster: &GangRoster,
) -> Result<String, ron::Error> {
    ron::ser::to_string_pretty(roster, ron::ser::PrettyConfig::default())
}

/// Write the edited gang to `assets/content/gangs/<gang_name>.ron` (C1).
///
/// Serializes the roster into the GTW-415 schema, creates the `content/gangs/` directory if it is
/// absent, and writes the file. Every fallible step is handled by LOGGING an `error!` and returning
/// early — it NEVER `unwrap`/`expect`/`panic`s (the no-panic rule). A success logs an `info!`
/// naming the written path.
///
/// The whole module is `#[cfg(debug_assertions)]`-gated (C3), so this filesystem write is never
/// compiled into a release binary.
fn write_gang_roster(name: &GangName, roster: &GangRoster) {
    let serialized = match serialize_roster(roster) {
        Ok(serialized) => serialized,
        Err(err) => {
            error!(
                "gang save: failed to serialize gang `{}`: {err}",
                name.as_str()
            );
            return;
        }
    };
    let path = gang_save_path(name);
    if let Some(dir) = path.parent()
        && let Err(err) = std::fs::create_dir_all(dir)
    {
        error!("gang save: failed to create `{}`: {err}", dir.display());
        return;
    }
    if let Err(err) = std::fs::write(&path, serialized) {
        error!("gang save: failed to write `{}`: {err}", path.display());
        return;
    }
    info!(
        "gang save: wrote gang `{}` to `{}`",
        name.as_str(),
        path.display()
    );
}

/// `Update` (gated `in_state(DebugEditor)`): writes the edited gang to disk on a "Save gang" press
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
pub(in crate::states::running::editor) fn save_gang_on_press(
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
    use gdtf_battle_sim::{
        Aim, ArmorName, Cool, GangMember, GangName, GangRegistry, GangRoster, GangerName, Grit,
        Reflexes, Speed, Strength, Toughness, WeaponName, ganger::Luck,
    };

    use super::{gang_file_name, serialize_roster};
    use crate::states::running::editor::model::EditableGang;

    /// A two-member edited gang's REGISTRY fixture — built from the fully-public sim
    /// [`GangRoster`] / [`GangMember`] types so the test does not reach the editor model's private
    /// fields. The magnitudes are arbitrary fixture data (NOT pinned shipped tuning): the test
    /// asserts the values SURVIVE the round-trip, never that they equal a shipped number.
    fn edited_registry() -> GangRegistry {
        let roster = GangRoster::new([
            GangMember {
                name:      GangerName::new("Edited Alpha".to_owned()),
                speed:     Speed::new(2.5),
                aim:       Aim::new(3.0),
                strength:  Strength::new(4.0),
                toughness: Toughness::new(11.0),
                reflexes:  Reflexes::new(3.5),
                cool:      Cool::new(6.0),
                grit:      Grit::new(18.0),
                luck:      Luck::new(1.0),
                armor:     ArmorName::new("flak_vest".to_owned()),
                weapon:    WeaponName::new("stub_pistol".to_owned()),
            },
            GangMember {
                name:      GangerName::new("Edited Bravo".to_owned()),
                speed:     Speed::new(4.0),
                aim:       Aim::new(5.0),
                strength:  Strength::new(2.0),
                toughness: Toughness::new(9.0),
                reflexes:  Reflexes::new(4.0),
                cool:      Cool::new(7.0),
                grit:      Grit::new(15.0),
                luck:      Luck::new(2.0),
                armor:     ArmorName::new("mesh_armor".to_owned()),
                weapon:    WeaponName::new("autogun".to_owned()),
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
    /// (the loader's `gang_key_from_stem`). The editor saves `<name>.ron` (no `.gang` infix), so
    /// the strip is a no-op and the key is the gang name unchanged.
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
        // The on-disk file name the save path would use — `<gang_name>.ron`.
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
        // … and key it into a registry by the file STEM minus a trailing `.gang` (the loader's
        // `gang_key_from_stem`), mirroring `build_gang_registry`.
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

    /// The save FILE NAME is `<gang_name>.ron` (the RESOLVED save path, user 2026-06-26) — and the
    /// loader's stem-keying recovers exactly the gang name from it (no `.gang` infix to strip).
    #[test]
    fn save_file_name_is_gang_name_dot_ron() {
        let name = GangName::new("goliaths".to_owned());
        assert_eq!(gang_file_name(&name), "goliaths.ron");
    }
}
