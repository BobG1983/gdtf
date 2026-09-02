//! The headless renderer apps every terrain-draw case starts from.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::error::warn,
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_assets::ContentFamilyAppExt;
use gdtf_battle_presenter::{CharacterRoles, TopDownAtlases, TopDownRendererPlugin};
use gdtf_battle_sim::{
    battle::BattleReady,
    occupancy_sync::TerrainPieceDestroyed,
    test_support::{
        test_armor_registry, test_gang_registry, test_melee_weapon_registry, test_terrain_registry,
        test_weapon_registry,
    },
};
use gdtf_content_families::{SpriteDefsFamily, sprites::SpriteDefRegistry};
use gdtf_test_utils::advance_until_resource_exists;

pub(crate) fn workspace_assets_root() -> PathBuf {
    let Some(root) = gdtf_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

// The shared renderer build both public builders call.
fn renderer_app_at(assets_root: &std::path::Path) -> App {
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
                file_path: assets_root.to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .add_message::<BattleReady>()
    .add_plugins(TopDownRendererPlugin);
    app.register_content_family::<SpriteDefsFamily>();
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app.insert_resource(test_terrain_registry());
    app.insert_resource(test_gang_registry());
    super::setup::register_setup_driver(&mut app);
    app.set_error_handler(warn);
    app
}

pub(crate) fn headless_renderer_app_at(assets_root: &std::path::Path) -> App {
    let mut app = renderer_app_at(assets_root);
    app.add_message::<TerrainPieceDestroyed>();
    app
}

pub(crate) fn headless_renderer_app() -> App {
    headless_renderer_app_at(&workspace_assets_root())
}

/// The same renderer app with no raw `TerrainPieceDestroyed` buffer, so only the played buffer
/// can drive a destruction stamp.
pub(crate) fn headless_renderer_app_without_raw_destroyed() -> App {
    renderer_app_at(&workspace_assets_root())
}

pub(crate) fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<SpriteDefRegistry>(app);
    advance_until_resource_exists::<TopDownAtlases>(app);
    advance_until_resource_exists::<CharacterRoles>(app);
}

pub(crate) fn write_sprite_def(root: &std::path::Path, file: &str, payload: &str) {
    let dir = root.join("content").join("sprites");
    let created = std::fs::create_dir_all(&dir);
    assert!(
        created.is_ok(),
        "creating the sprites dir must succeed: {created:?}"
    );
    let written = std::fs::write(dir.join(file), payload);
    assert!(written.is_ok(), "writing {file} must succeed: {written:?}");
}
