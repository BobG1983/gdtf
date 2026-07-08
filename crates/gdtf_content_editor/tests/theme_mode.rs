//! GTW-662 C2/A2: the THEME mode's REAL fs round-trip — author a theme in the form model,
//! save it through the REAL root-parameterized write (`write_theme_in`) into a `TempDir`
//! assets root (the GTW-555 pattern — the shipped `assets/` tree is NEVER written), then
//! boot the REAL editor app rooted at that directory and assert the actual
//! `ThemeDefsFamily` folder walk loads the saved theme back structurally identical.
//!
//! This is the FIRST test to exercise `write_theme`'s filesystem half (the GTW-653 census
//! finding): before GTW-662 the writer had no `TempDir` seam, so its fs half was untestable
//! without polluting the version-controlled `assets/` tree. Only `content/terrain/` is
//! materialized here (one theme file), so every other family fails closed to its empty
//! registry (the no-strand guarantee, the `armor_mode.rs` precedent); the theme's palette
//! UUIDs deliberately resolve nowhere — the GTW-582 reference-integrity pass RECORDS
//! dangling-ref findings, it never rejects a loaded theme. `assert!` + `let … else` keep
//! the test panic-free per the workspace lints.

#![cfg(debug_assertions)]

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
use gdtf_battle_sim::{level::UuidThemeRegistry, terrain::def::TerrainUuid};
use gdtf_content_editor::{
    EditorState, MapEditorPlugin, ThemeDraft, draft_to_theme_def, write_theme_in,
};
use gdtf_test_utils::advance_until;

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET (not a timing budget) —
/// the test polls the `EditorState::Editing` SIGNAL.
const MAX_UPDATES: u32 = 10_000;

/// A stable [`TerrainUuid`] fixture for the theme's default floor — mechanism, not balance.
const FLOOR: TerrainUuid = TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0662_0000_0001));

/// A second palette member so the saved `terrain` list is more than the floor alone.
const COVER: TerrainUuid = TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0662_0000_0002));

/// The real editor app rooted at an ARBITRARY assets directory (the `armor_mode.rs`
/// recipe): only `content/terrain/` is materialized by this test (one theme file), so
/// every other family fails closed to its empty registry (the no-strand guarantee) while
/// the theme walk loads the REAL saved file.
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
         UuidThemeRegistry) did not resolve or fall back",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// The authored theme the test builds through the REAL form-model mutators: a named theme
/// with a two-member palette and the floor chosen from it (so `validate_for_save`'s C6
/// default-floor rule passes on the real save path).
fn edited_draft() -> ThemeDraft {
    let mut draft = ThemeDraft::new_theme();
    draft.set_display_name("Tempdir Theme".to_owned());
    draft.toggle_terrain(FLOOR);
    draft.toggle_terrain(COVER);
    draft.set_default_floor(FLOOR);
    draft
}

/// GTW-662 — author → save (the REAL `write_theme_in` into a `TempDir` assets root) → load
/// through the REAL `ThemeDefsFamily` folder walk → the registry holds the SAME def
/// (structural equality: display name, palette, default floor, payload-carried UUID key).
#[test]
fn saved_theme_round_trips_through_the_real_theme_family_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    // SAVE through the real root-parameterized write. The expected def is the SAME
    // projection the write serializes (`draft_to_theme_def`, keyed by the draft's minted key).
    let draft = edited_draft();
    let key = draft.key();
    let expected = draft_to_theme_def(&draft, key);
    let written = write_theme_in(dir.path(), &draft, key);
    assert!(
        written.is_ok(),
        "the real theme write must succeed: {:?}",
        written.as_ref().err(),
    );
    // The slugged per-theme layout the GTW-487 loader walks: dir + stem both key on the slug.
    if let Ok(path) = &written {
        assert!(
            path.ends_with("content/terrain/tempdir_theme/tempdir_theme.terrain_theme.ron"),
            "the saved theme must land on the family's slugged path; got {}",
            path.display(),
        );
    }

    // RELOAD through the real editor Load pass rooted at the TempDir.
    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    // The REAL folder walk keyed the saved file by its PAYLOAD UUID (the theme family is
    // payload-keyed — the filename stem is cosmetic) and loaded the SAME def.
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
