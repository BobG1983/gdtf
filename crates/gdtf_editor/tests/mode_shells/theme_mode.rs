//! C2/A2: the THEME mode's REAL fs round-trip — author a theme in the form model,
//! without polluting the version-controlled `assets/` tree. Only `content/terrain/` is
#[cfg(not(feature = "mcp"))]
compile_error!("the mcp test suites need the host package's `mcp` feature");

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
use cobalt_test_utils::advance_until;
use gdtf_battle_sim::{level::UuidThemeRegistry, terrain::def::TerrainUuid};
use gdtf_editor::{EditorState, MapEditorPlugin, ThemeDraft, draft_to_theme_def, write_theme_in};

const FLOOR: TerrainUuid = TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0662_0000_0001));

const COVER: TerrainUuid = TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0662_0000_0002));

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
    advance_until(app, |app| {
        app.world()
            .get_resource::<State<EditorState>>()
            .is_some_and(|s| *s.get() == EditorState::Editing)
    });
    for _ in 0..4 {
        app.update();
    }
}

fn edited_draft() -> ThemeDraft {
    let mut draft = ThemeDraft::new_theme();
    draft.set_display_name("Tempdir Theme".to_owned());
    draft.toggle_terrain(FLOOR);
    draft.toggle_terrain(COVER);
    draft.set_default_floor(FLOOR);
    draft
}

#[test]
fn saved_theme_round_trips_through_the_real_theme_family_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let draft = edited_draft();
    let key = draft.key();
    let expected = draft_to_theme_def(&draft, key);
    let written = write_theme_in(dir.path(), &draft, key);
    assert!(
        written.is_ok(),
        "the real theme write must succeed: {:?}",
        written.as_ref().err(),
    );
    if let Ok(path) = &written {
        assert!(
            path.ends_with("content/terrain/tempdir_theme/tempdir_theme.terrain_theme.ron"),
            "the saved theme must land on the family's slugged path; got {}",
            path.display(),
        );
    }

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let registry = app.world().get_resource::<UuidThemeRegistry>();
    assert!(registry.is_some(), "the UuidThemeRegistry must resolve");
    let Some(registry) = registry else { return };
    assert_eq!(
        registry.def(&key),
        Some(&expected),
        "the reloaded theme must equal the saved def (display name / palette / default \
         floor / key) — the payload-UUID round-trip through the REAL ThemeDefsFamily loader",
    );
}
