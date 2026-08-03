use std::path::Path;

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::AssetPlugin,
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_assets::ContentFolderHandle;
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::{InjuryRegistry, InjuryTables},
    level::UuidThemeRegistry,
    terrain::def::TerrainDefRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_editor::{EditorState, MapEditorPlugin};
use gdtf_content_families::{
    ArmorFamily, AttachmentsFamily, GangsFamily, MeleeWeaponsFamily, SpriteDefsFamily,
    TerrainDefsFamily, ThemeDefsFamily, WeaponsFamily, sprites::SpriteDefRegistry,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

const MAX_UPDATES: u32 = 10_000;

fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

fn editor_app_with_asset_root(root: &Path) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            }),
    );
    app.set_error_handler(warn);
    app.add_plugins(MapEditorPlugin);
    app
}

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
         resolve (or fall back) every gate resource",
    );
    for _ in 0..4 {
        app.update();
    }
}

#[test]
fn load_pass_resolves_all_ten_resources_through_the_real_seams() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let world = app.world();
    assert!(
        world
            .get_resource::<WeaponRegistry>()
            .is_some_and(|r| !r.is_empty()),
        "the WeaponRegistry must resolve NON-empty through the shared WeaponsFamily",
    );
    assert!(
        world
            .get_resource::<ArmorRegistry>()
            .is_some_and(|r| !r.is_empty()),
        "the ArmorRegistry must resolve NON-empty through the shared ArmorFamily",
    );
    assert!(
        world
            .get_resource::<TerrainDefRegistry>()
            .is_some_and(|r| !r.is_empty()),
        "the TerrainDefRegistry must resolve NON-empty through the shared TerrainDefsFamily",
    );
    assert!(
        world
            .get_resource::<UuidThemeRegistry>()
            .is_some_and(|r| !r.is_empty()),
        "the UuidThemeRegistry must resolve NON-empty through the shared ThemeDefsFamily",
    );
    assert!(
        world
            .get_resource::<GangRegistry>()
            .is_some_and(|r| !r.is_empty()),
        "the GangRegistry must resolve NON-empty through the shared GangsFamily (GTW-636)",
    );
    assert!(
        world
            .get_resource::<MeleeWeaponRegistry>()
            .is_some_and(|r| !r.is_empty()),
        "the MeleeWeaponRegistry must resolve NON-empty through the shared MeleeWeaponsFamily \
         (GTW-636)",
    );
    assert!(
        world
            .get_resource::<InjuryRegistry>()
            .is_some_and(|r| !r.is_empty()),
        "the InjuryRegistry must resolve NON-empty through the editor's bespoke injuries pass \
         (GTW-654)",
    );
    assert!(
        world
            .get_resource::<InjuryTables>()
            .is_some_and(|t| !t.is_empty()),
        "the InjuryTables must resolve NON-empty through the editor's bespoke injuries pass \
         (GTW-654)",
    );
    assert!(
        world
            .get_resource::<SpriteDefRegistry>()
            .is_some_and(|r| !r.is_empty()),
        "the SpriteDefRegistry must resolve NON-empty through the shared SpriteDefsFamily \
         (GTW-663)",
    );
    assert!(
        world
            .get_resource::<AttachmentRegistry>()
            .is_some_and(|r| !r.is_empty()),
        "the AttachmentRegistry must resolve NON-empty through the shared AttachmentsFamily \
         (GTW-669)",
    );

    assert_seam_handles_persist(world);
}

fn assert_seam_handles_persist(world: &World) {
    assert!(
        world
            .get_resource::<ContentFolderHandle<WeaponsFamily>>()
            .is_some(),
        "the weapons ContentFolderHandle must persist past Load",
    );
    assert!(
        world
            .get_resource::<ContentFolderHandle<ArmorFamily>>()
            .is_some(),
        "the armor ContentFolderHandle must persist past Load",
    );
    assert!(
        world
            .get_resource::<ContentFolderHandle<TerrainDefsFamily>>()
            .is_some(),
        "the terrain-defs ContentFolderHandle must persist past Load",
    );
    assert!(
        world
            .get_resource::<ContentFolderHandle<ThemeDefsFamily>>()
            .is_some(),
        "the theme-defs ContentFolderHandle must persist past Load",
    );
    assert!(
        world
            .get_resource::<ContentFolderHandle<GangsFamily>>()
            .is_some(),
        "the gangs ContentFolderHandle must persist past Load (GTW-636)",
    );
    assert!(
        world
            .get_resource::<ContentFolderHandle<MeleeWeaponsFamily>>()
            .is_some(),
        "the melee-weapons ContentFolderHandle must persist past Load (GTW-636)",
    );
    assert!(
        world
            .get_resource::<ContentFolderHandle<SpriteDefsFamily>>()
            .is_some(),
        "the sprite-defs ContentFolderHandle must persist past Load (GTW-663)",
    );
    assert!(
        world
            .get_resource::<ContentFolderHandle<AttachmentsFamily>>()
            .is_some(),
        "the attachments ContentFolderHandle must persist past Load (GTW-669)",
    );
}

#[test]
fn failed_asset_root_falls_back_and_still_reaches_editing() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the empty asset root must succeed");
    let Ok(dir) = dir else { return };

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    assert_eq!(
        world
            .get_resource::<WeaponRegistry>()
            .map(WeaponRegistry::is_empty),
        Some(true),
        "a Failed weapons folder must fail closed to the EMPTY WeaponRegistry",
    );
    assert_eq!(
        world
            .get_resource::<ArmorRegistry>()
            .map(ArmorRegistry::is_empty),
        Some(true),
        "a Failed armor folder must fail closed to the EMPTY ArmorRegistry",
    );
    assert_eq!(
        world
            .get_resource::<TerrainDefRegistry>()
            .map(TerrainDefRegistry::is_empty),
        Some(true),
        "a Failed terrain folder must fail closed to the EMPTY TerrainDefRegistry",
    );
    assert_eq!(
        world
            .get_resource::<UuidThemeRegistry>()
            .map(UuidThemeRegistry::is_empty),
        Some(true),
        "a Failed terrain folder must fail closed to the EMPTY UuidThemeRegistry",
    );
    assert_eq!(
        world
            .get_resource::<GangRegistry>()
            .map(GangRegistry::is_empty),
        Some(true),
        "a Failed gangs folder must fail closed to the EMPTY GangRegistry (GTW-636)",
    );
    assert_eq!(
        world
            .get_resource::<MeleeWeaponRegistry>()
            .map(MeleeWeaponRegistry::is_empty),
        Some(true),
        "a Failed melee folder must fail closed to the EMPTY MeleeWeaponRegistry (GTW-636)",
    );
    assert_eq!(
        world
            .get_resource::<SpriteDefRegistry>()
            .map(SpriteDefRegistry::is_empty),
        Some(true),
        "a Failed sprites folder must fail closed to the EMPTY SpriteDefRegistry (GTW-663)",
    );
    assert_eq!(
        world
            .get_resource::<AttachmentRegistry>()
            .map(AttachmentRegistry::is_empty),
        Some(true),
        "a Failed attachments folder must fail closed to the EMPTY AttachmentRegistry (GTW-669)",
    );
    assert_eq!(
        world
            .get_resource::<InjuryRegistry>()
            .map(InjuryRegistry::is_empty),
        Some(true),
        "a Failed injuries folder must fail closed to the EMPTY InjuryRegistry (GTW-654)",
    );
    assert_eq!(
        world
            .get_resource::<InjuryTables>()
            .map(InjuryTables::is_empty),
        Some(true),
        "a Failed injuries folder must fail closed to the EMPTY InjuryTables (GTW-654)",
    );
}
