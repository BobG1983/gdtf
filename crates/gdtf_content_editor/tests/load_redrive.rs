//! GTW-579 AC-3 headless pins: an `AssetEvent::Modified` on a member of EACH of
//! the editor's FOUR folder families rebuilds the corresponding registry IN
//! PLACE, AFTER `Load` exited — through the REAL GTW-570 content-family systems
//! [`MapEditorPlugin`] registers (no editor-local redrive survives GTW-579),
//! from the persistent whole-session [`ContentFolderHandle`]s (GTW-533).
//!
//! Each test drives the real editor app (no-renderer harness, live workspace
//! `assets/` root) to `Editing`, hot-edits ONE loaded member's in-memory
//! payload, fires the SAME `AssetEvent::Modified` the file-watcher emits, and
//! asserts the registry reflects the edit with NO restart — the pre-GTW-579
//! in-crate redrive pins, rewired onto the shared content-family path and extended to all four
//! families.

use bevy::{
    asset::{AssetEvent, AssetServer, Assets},
    prelude::*,
};
use gdtf_assets::{ContentFamily, ContentFolderHandle, RonAsset};
use gdtf_battle_sim::{
    armor::{ArmorName, ArmorRegistry, ArmorSpec},
    level::{ThemeDisplayName, UuidThemeDef, UuidThemeRegistry},
    terrain::def::{TerrainDef, TerrainDefRegistry, TerrainDisplayName},
    weapon::{WeaponName, WeaponRegistry, WeaponSpec},
};
use gdtf_content_editor::{EditorState, MapEditorPlugin};
use gdtf_content_families::{ArmorFamily, TerrainDefsFamily, ThemeDefsFamily, WeaponsFamily};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

/// A generous frame cap for the async Load resolve — a SAFETY NET, not a timing
/// budget (the tests poll the `EditorState::Editing` signal).
const MAX_UPDATES: u32 = 10_000;

/// A small cap for the post-edit redrive settle (polled by signal, never slept).
const REDRIVE_UPDATES: u32 = 100;

/// The real editor app on the no-renderer `DefaultPlugins` UI harness (live
/// `AssetServer` rooted at the workspace `assets/`).
fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

/// Drives the app until [`EditorState::Editing`] (Load exited), then a few
/// settle frames so the `OnEnter(Editing)` command flushes apply.
fn advance_to_editing(app: &mut App) {
    let reached = advance_until(
        app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|s| *s.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — the registered Load pass did not \
         resolve the gate resources",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// A stub_pistol-shaped [`WeaponSpec`] with the given `damage`, parsed from
/// inline RON; [`None`] (assert-fail) on a parse error, never a denied `unwrap`.
fn weapon_spec(damage: i32) -> Option<WeaponSpec> {
    let ron = format!(
        "(base_spread: 0.10, accuracy: 1.0, kickback: 0.05, fatal_bias: 0.0, \
         damage: {damage}, punch: 2, shred: 1, damage_type: Kinetic, \
         magazine: (size: 12, reload_tu: 12), \
         fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.30, shots: 1)], \
         stable: false, handedness: OneHanded)",
    );
    let parsed = ron::de::from_str::<WeaponSpec>(&ron);
    assert!(
        parsed.is_ok(),
        "weapon fixture must parse: {:?}",
        parsed.as_ref().err()
    );
    parsed.ok()
}

/// A flak_vest-shaped [`ArmorSpec`] whose torso protection is the given distinct
/// sentinel; [`None`] (assert-fail) on a parse error, never a denied `unwrap`.
fn armor_spec(torso_protection: i32) -> Option<ArmorSpec> {
    let ron = format!(
        "(head:      (floor: 2, protection: 3, integrity: 50, hardness: 1, armor_type: Flak), \
          torso:     (floor: 2, protection: {torso_protection}, integrity: 60, hardness: 1, armor_type: Flak), \
          left_arm:  (floor: 1, protection: 2, integrity: 45, hardness: 1, armor_type: Flak), \
          right_arm: (floor: 1, protection: 2, integrity: 45, hardness: 1, armor_type: Flak), \
          left_leg:  (floor: 1, protection: 2, integrity: 50, hardness: 1, armor_type: Flak), \
          right_leg: (floor: 1, protection: 2, integrity: 50, hardness: 1, armor_type: Flak))",
    );
    let parsed = ron::de::from_str::<ArmorSpec>(&ron);
    assert!(
        parsed.is_ok(),
        "armor fixture must parse: {:?}",
        parsed.as_ref().err()
    );
    parsed.ok()
}

/// Asserts the family's [`ContentFolderHandle`] survived past `Load` (GTW-533
/// whole-session persistence), then fires the given `Modified` id.
fn assert_handle_persists_and_fire<F: gdtf_assets::ContentFamily>(
    app: &mut App,
    id: bevy::asset::AssetId<RonAsset<F::Spec>>,
) {
    assert!(
        app.world()
            .get_resource::<ContentFolderHandle<F>>()
            .is_some(),
        "the loader's persistent ContentFolderHandle must survive past Load (GTW-533)",
    );
    app.world_mut().write_message(AssetEvent::Modified { id });
}

/// AC-3 (weapons): a `Modified` for the loaded `stub_pistol.weapon.ron` member
/// rebuilds the [`WeaponRegistry`] in place — keyed by file stem — reflecting
/// the edited damage, after `Load` exited.
#[test]
fn modified_weapon_member_rebuilds_weapon_registry_after_load() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<WeaponSpec>>("content/weapons/ranged/stub_pistol.weapon.ron");
    let Some(edited) = weapon_spec(97) else {
        return;
    };
    {
        let mut specs = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<WeaponSpec>>>();
        let asset = specs.get_mut(&handle);
        assert!(
            asset.is_some(),
            "the stub_pistol member must be resident once Editing is reached",
        );
        if let Some(mut asset) = asset {
            **asset = edited;
        }
    }
    assert_handle_persists_and_fire::<WeaponsFamily>(&mut app, handle.id());

    let key = WeaponName::new("stub_pistol".to_owned());
    let rebuilt = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<WeaponRegistry>()
                .and_then(|r| r.spec(&key).map(|s| *s.damage))
                == Some(97)
        },
        REDRIVE_UPDATES,
    );
    assert!(
        rebuilt,
        "the Load redrive must rebuild the WeaponRegistry in place with the edited damage",
    );
}

/// AC-3 (armor): a `Modified` for the loaded `flak_vest.armor.ron` member
/// rebuilds the [`ArmorRegistry`] in place — keyed by file stem — reflecting the
/// edited spec, after `Load` exited.
#[test]
fn modified_armor_member_rebuilds_armor_registry_after_load() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<ArmorSpec>>("content/armor/flak_vest.armor.ron");
    let Some(edited) = armor_spec(99) else { return };
    {
        let mut specs = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<ArmorSpec>>>();
        let asset = specs.get_mut(&handle);
        assert!(
            asset.is_some(),
            "the flak_vest member must be resident once Editing is reached",
        );
        if let Some(mut asset) = asset {
            **asset = edited;
        }
    }
    assert_handle_persists_and_fire::<ArmorFamily>(&mut app, handle.id());

    let key = ArmorName::new("flak_vest".to_owned());
    let rebuilt = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<ArmorRegistry>()
                .and_then(|r| r.spec(&key))
                == armor_spec(99).as_ref()
        },
        REDRIVE_UPDATES,
    );
    assert!(
        rebuilt,
        "the Load redrive must rebuild the ArmorRegistry in place with the edited spec",
    );
}

/// AC-3 (terrain defs): a `Modified` for a loaded `*.terrain_def.ron` member
/// rebuilds the [`TerrainDefRegistry`] in place — keyed by the def's OWN UUID —
/// reflecting the edited display name, after `Load` exited.
#[test]
fn modified_terrain_def_member_rebuilds_terrain_registry_after_load() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let handle = app
        .world()
        .resource::<AssetServer>()
        // GTW-634 A1: the folder segment is DERIVED from the family's owning const.
        .load::<RonAsset<TerrainDef>>(format!(
            "{}/underhive/scrap_barricade.terrain_def.ron",
            TerrainDefsFamily::FOLDER
        ));
    // Edit IN PLACE under the def's own (payload) key, so the assertion needs no
    // shipped-UUID pin.
    let key = {
        let mut defs = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<TerrainDef>>>();
        let asset = defs.get_mut(&handle);
        assert!(
            asset.is_some(),
            "the scrap_barricade member must be resident once Editing is reached",
        );
        let Some(mut asset) = asset else { return };
        asset.display_name = TerrainDisplayName::new("GTW-579 Edited Terrain".to_owned());
        asset.key
    };
    assert_handle_persists_and_fire::<TerrainDefsFamily>(&mut app, handle.id());

    let rebuilt = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<TerrainDefRegistry>()
                .and_then(|r| r.def(&key).map(|d| (*d.display_name).clone()))
                == Some("GTW-579 Edited Terrain".to_owned())
        },
        REDRIVE_UPDATES,
    );
    assert!(
        rebuilt,
        "the Load redrive must rebuild the TerrainDefRegistry in place, keyed by the def UUID, \
         with the edited display name — no restart",
    );
}

/// AC-3 (theme defs): a `Modified` for a loaded `*.terrain_theme.ron` member
/// rebuilds the [`UuidThemeRegistry`] in place — keyed by the theme's OWN UUID —
/// reflecting the edited display name, after `Load` exited.
#[test]
fn modified_theme_def_member_rebuilds_theme_registry_after_load() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<UuidThemeDef>>(format!(
            "{}/underhive/underhive.terrain_theme.ron",
            ThemeDefsFamily::FOLDER
        ));
    let key = {
        let mut defs = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<UuidThemeDef>>>();
        let asset = defs.get_mut(&handle);
        assert!(
            asset.is_some(),
            "the underhive theme member must be resident once Editing is reached",
        );
        let Some(mut asset) = asset else { return };
        asset.display_name = ThemeDisplayName::new("GTW-579 Edited Theme".to_owned());
        asset.key
    };
    assert_handle_persists_and_fire::<ThemeDefsFamily>(&mut app, handle.id());

    let rebuilt = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<UuidThemeRegistry>()
                .and_then(|r| r.def(&key).map(|d| (*d.display_name).clone()))
                == Some("GTW-579 Edited Theme".to_owned())
        },
        REDRIVE_UPDATES,
    );
    assert!(
        rebuilt,
        "the Load redrive must rebuild the UuidThemeRegistry in place, keyed by the theme UUID, \
         with the edited display name — no restart",
    );
}
