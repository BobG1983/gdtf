//! Sprite mode: author a sprite def and round-trip through save and reload.
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
use gdtf_content_families::sprites::{
    SpriteDefRegistry, SpriteFacing, SpriteFps, SpriteImagePath, SpriteName, SpritePx, SpriteRect,
    SpriteSource,
};
use gdtf_editor::{
    EditorState, MapEditorPlugin, SpriteDraft, draft_to_sprite_def, write_sprite_in,
};
use gdtf_test_utils::advance_until;

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

fn edited_draft() -> SpriteDraft {
    let mut draft = SpriteDraft::new_sprite();
    draft.set_name("tempdir_glyph".to_owned());
    draft.set_base_source(SpriteSource::Sheet {
        sheet: SpriteImagePath::new("sprites/alt_tileset_terrain.png".to_owned()),
        rect:  SpriteRect {
            x: SpritePx::new(32),
            y: SpritePx::new(16),
            w: SpritePx::new(16),
            h: SpritePx::new(16),
        },
    });
    draft.set_anchor(SpritePx::new(8), SpritePx::new(16));
    draft.set_facing_override(
        SpriteFacing::East,
        Some(SpriteSource::File(SpriteImagePath::new(
            "sprites/east_variant.png".to_owned(),
        ))),
    );
    draft.enable_animation();
    draft.set_fps(SpriteFps::new(2.5));
    draft.add_frame();
    draft.set_frame(
        1,
        SpriteSource::Sheet {
            sheet: SpriteImagePath::new("sprites/alt_tileset_terrain.png".to_owned()),
            rect:  SpriteRect {
                x: SpritePx::new(48),
                y: SpritePx::new(16),
                w: SpritePx::new(16),
                h: SpritePx::new(16),
            },
        },
    );
    draft
}

#[test]
fn saved_sprite_round_trips_through_the_real_sprite_defs_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let draft = edited_draft();
    let (name, def) = draft_to_sprite_def(&draft);
    let written = write_sprite_in(dir.path(), &name, &def);
    assert!(
        written.is_ok(),
        "the real sprite write must succeed: {:?}",
        written.as_ref().err(),
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    assert!(
        world.get_resource::<SpriteDraft>().is_some(),
        "the SpriteDraft must be seeded OnEnter(Editing)",
    );

    let registry = world.get_resource::<SpriteDefRegistry>();
    assert!(registry.is_some(), "the SpriteDefRegistry must resolve");
    let Some(registry) = registry else { return };
    let reloaded = registry.def(&SpriteName::new("tempdir_glyph".to_owned()));
    assert_eq!(
        reloaded,
        Some(&def),
        "the reloaded sprite must equal the saved def (source sheet+rect / anchor / the \
         East facing override / the 2-frame 2.5fps animation) — the stem-key \
         round-trip through the REAL loader",
    );
}
