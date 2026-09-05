//! Load redrive: edited content-family members rebuild registries in place.
use bevy::{
    asset::{AssetEvent, AssetServer, Assets},
    prelude::*,
};
use cobalt_ron_assets::RonAsset;
use cobalt_test_utils::{UiTestAppBuilder, advance_until};
use gdtf_assets::{ContentFamily, ContentFolderHandle};
use gdtf_battle_sim::{
    armor::{ArmorName, ArmorRegistry, ArmorSpec},
    level::{ThemeDisplayName, UuidThemeDef, UuidThemeRegistry},
    terrain::def::{TerrainDef, TerrainDefRegistry, TerrainDisplayName},
    weapon::{WeaponName, WeaponRegistry, WeaponSpec},
};
use gdtf_content_families::{ArmorFamily, TerrainDefsFamily, ThemeDefsFamily, WeaponsFamily};
use gdtf_editor::{EditorState, MapEditorPlugin};

fn editor_app() -> App {
    let mut app = UiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

fn advance_to_editing(app: &mut App) {
    advance_until(app, |app| {
        app.world()
            .get_resource::<State<EditorState>>()
            .is_some_and(|s| *s.get() == EditorState::Editing)
    });
    for _ in 0..4 {
        app.update();
    }
}

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

fn assert_handle_persists_and_fire<F: gdtf_assets::ContentFamily>(
    app: &mut App,
    id: bevy::asset::AssetId<RonAsset<F::Spec>>,
) {
    assert!(
        app.world()
            .get_resource::<ContentFolderHandle<F>>()
            .is_some(),
        "the loader's persistent ContentFolderHandle must survive past Load",
    );
    app.world_mut().write_message(AssetEvent::Modified { id });
}

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
    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<WeaponRegistry>()
            .and_then(|r| r.spec(&key).map(|s| *s.damage))
            == Some(97)
    });
}

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
    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<ArmorRegistry>()
            .and_then(|r| r.spec(&key))
            == armor_spec(99).as_ref()
    });
}

#[test]
fn modified_terrain_def_member_rebuilds_terrain_registry_after_load() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<TerrainDef>>(format!(
            "{}/underhive/scrap_barricade.terrain_def.ron",
            TerrainDefsFamily::FOLDER
        ));
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
        asset.display_name = TerrainDisplayName::new("Edited Terrain".to_owned());
        asset.key
    };
    assert_handle_persists_and_fire::<TerrainDefsFamily>(&mut app, handle.id());

    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<TerrainDefRegistry>()
            .and_then(|r| r.def(&key).map(|d| (*d.display_name).clone()))
            == Some("Edited Terrain".to_owned())
    });
}

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
        asset.display_name = ThemeDisplayName::new("Edited Theme".to_owned());
        asset.key
    };
    assert_handle_persists_and_fire::<ThemeDefsFamily>(&mut app, handle.id());

    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<UuidThemeRegistry>()
            .and_then(|r| r.def(&key).map(|d| (*d.display_name).clone()))
            == Some("Edited Theme".to_owned())
    });
}
