//! GTW-664 C4/A2: the SPRITE mode's REAL round-trip — author a sprite def (anchor +
//! facings + animation, every ruled field) in the form model, save it through the REAL
//! root-parameterized write (`write_sprite_in`) into a `TempDir` assets root (the
//! GTW-555 pattern — the shipped `assets/` tree is NEVER written), then boot the REAL
//! editor app rooted at that directory and assert the actual `SpriteDefsFamily` folder
//! walk loads the saved def back structurally identical.
//!
//! Also pins the GTW-664 lifecycle riders: the editor reaches `Editing` with the
//! `SpriteDefRegistry` gate resource present and the state-scoped `SpriteDraft` seeded
//! (salvage / fallback behavior itself is the seam's parameterized family contract —
//! `register_content_family::<SpriteDefsFamily>` inherits it, no per-family re-pin
//! here). Re-arm on save is likewise NOT re-pinned: the editor's validation watch set
//! already contains the `SpriteDefRegistry` (GTW-663, `validate/rearm.rs`), so a saved
//! member's registry rebuild re-runs the `graphic_name` edge by that landed contract.

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
use gdtf_content_editor::{
    EditorState, MapEditorPlugin, SpriteDraft, draft_to_sprite_def, write_sprite_in,
};
use gdtf_content_families::sprites::{
    SpriteDefRegistry, SpriteFacing, SpriteFps, SpriteImagePath, SpriteName, SpritePx, SpriteRect,
    SpriteSource,
};
use gdtf_test_utils::advance_until;

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET (not a timing budget) —
/// the test polls the `EditorState::Editing` SIGNAL.
const MAX_UPDATES: u32 = 10_000;

/// The real editor app rooted at an ARBITRARY assets directory (the `armor_mode.rs`
/// recipe): only `content/sprites/` is materialized by this test, so every other family
/// fails closed to its empty registry (the no-strand guarantee) while the sprite walk
/// loads the REAL saved file.
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
            // Headless-test noise suppression (GTW-139): the deliberate
            // failure-path asset errors of the unmaterialized families stay quiet.
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
    // Bevy 0.19 routes a FAILED system-param validation to the global error handler
    // (default panics); with no render backend some render-provided params cannot
    // validate. `warn` restores the skip-with-a-log behavior (the shared harness
    // precedent).
    app.set_error_handler(warn);
    app.add_plugins(MapEditorPlugin);
    app
}

/// Drives the app until [`EditorState::Editing`], then a few settle frames so the
/// `OnEnter(Editing)` command flushes apply before the assertions read.
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
        "the editor never reached EditorState::Editing — the Load gate (including the \
         SpriteDefRegistry) did not resolve or fall back",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// The edited sprite the test authors through the REAL form-model mutators: every ruled
/// field populated — a sheet-cut base source, a non-center anchor, one facing override,
/// and a two-frame animation at a non-default rate (so a dropped field cannot
/// round-trip).
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

/// GTW-664 — create → save (the REAL write into a `TempDir` assets root) → load through
/// the REAL `SpriteDefsFamily` folder walk → the registry holds the SAME def
/// (structural equality across source / anchor / facings / animation), with the
/// `Editing` gate + the scoped `SpriteDraft` seed along for the ride.
#[test]
fn saved_sprite_round_trips_through_the_real_sprite_defs_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    // SAVE through the real root-parameterized write.
    let draft = edited_draft();
    let (name, def) = draft_to_sprite_def(&draft);
    let written = write_sprite_in(dir.path(), &name, &def);
    assert!(
        written.is_ok(),
        "the real sprite write must succeed: {:?}",
        written.as_ref().err(),
    );

    // RELOAD through the real editor Load pass rooted at the TempDir.
    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    // The state-scoped SPRITE draft seeded on entering Editing (bevy-traps #1 via the
    // GTW-575 seam).
    assert!(
        world.get_resource::<SpriteDraft>().is_some(),
        "the SpriteDraft must be seeded OnEnter(Editing)",
    );

    // The REAL folder walk keyed the saved file by its stem and loaded the SAME def.
    let registry = world.get_resource::<SpriteDefRegistry>();
    assert!(registry.is_some(), "the SpriteDefRegistry must resolve");
    let Some(registry) = registry else { return };
    let reloaded = registry.def(&SpriteName::new("tempdir_glyph".to_owned()));
    assert_eq!(
        reloaded,
        Some(&def),
        "the reloaded sprite must equal the saved def (source sheet+rect / anchor / the \
         East facing override / the 2-frame 2.5fps animation) — the GTW-663 stem-key \
         round-trip through the REAL loader",
    );
}
