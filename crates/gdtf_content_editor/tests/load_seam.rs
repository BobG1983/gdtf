//! GTW-579 headless pins for the editor's seam-registered `Load` pass.
//!
//! AC-2: driving the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins`
//! harness (live workspace `assets/` root) reaches [`EditorState::Editing`] with
//! ALL SIX resolved resources present — through the actual GTW-570 content-family
//! and GTW-564 hot-RON seam registrations, not an editor-local mirror — and the
//! seam's persistent handles survive past `Load` (the GTW-533 whole-session
//! persistence the live hot-reload rides).
//!
//! AC-4 (ADR-0003): pointing the SAME app at an EMPTY asset root fails every
//! load, and the editor STILL transitions to `Editing` — the theme + tile-role
//! chains fall back to their const defaults (the editor-owned policy attached at
//! registration) and the four folder families fail closed to EMPTY registries.

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
use gdtf_assets::{ContentFolderHandle, HotRonHandle};
use gdtf_battle_presenter::{TileIndex, TileRoles};
use gdtf_battle_sim::{
    armor::ArmorRegistry, level::UuidThemeRegistry, terrain::def::TerrainDefRegistry,
    weapon::WeaponRegistry,
};
use gdtf_content_editor::{EditorState, MapEditorPlugin};
use gdtf_content_families::{ArmorFamily, TerrainDefsFamily, ThemeDefsFamily, WeaponsFamily};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec};

/// A generous frame cap: the async asset loads under parallel `cargo` contention
/// take a non-deterministic number of frames, so this is a SAFETY NET (not a
/// timing budget) — the tests poll the `EditorState::Editing` SIGNAL.
const MAX_UPDATES: u32 = 10_000;

/// The real editor app on the no-renderer `DefaultPlugins` UI harness (live
/// `AssetServer` rooted at the workspace `assets/`) — the `state_scoped_resources`
/// recipe.
fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

/// The real editor app rooted at an ARBITRARY assets directory — the AC-4
/// failure-path harness (an empty root fails every load). Mirrors the
/// `no_renderer.rs` plugin recipe the shared harness uses; local to this test
/// because the shared builder deliberately pins the workspace root.
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
            // Headless-test noise suppression (GTW-139): no global tracing
            // subscriber, so the DELIBERATE failure-path asset errors this test
            // exercises do not print; the other three plugins probe a missing
            // window / RenderApp / audio device unused here.
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
    // Bevy 0.19 routes a FAILED system-param validation to the global error
    // handler (default panics); with no render backend some render-provided
    // params cannot validate. `warn` restores the skip-with-a-log behavior
    // (the shared harness precedent).
    app.set_error_handler(warn);
    app.add_plugins(MapEditorPlugin);
    app
}

/// Drives the app until [`EditorState::Editing`], then a few settle frames so
/// the `OnEnter(Editing)` command flushes apply before the assertions read.
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
        "the editor never reached EditorState::Editing — the seam-registered Load pass did not \
         resolve (or fall back) every gate resource",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// AC-2: the real seam path resolves the editor's whole gate set — the app
/// reaches `Editing` with the theme, the four folder registries (each NON-empty,
/// so the shipped content genuinely resolved — no count pins), and the tile-role
/// table all present, plus the six PERSISTENT seam handles (GTW-533 C4a: no
/// `OnExit(Load)` cleanup exists, so the live hot-reload substrate survives).
#[test]
fn load_pass_resolves_all_six_resources_through_the_real_seams() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let world = app.world();
    assert!(
        world.get_resource::<GdtfTheme>().is_some(),
        "the GdtfTheme must resolve through the published theme hot-RON chain",
    );
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
        world.get_resource::<TileRoles>().is_some(),
        "the TileRoles table must resolve through the presenter's published hot-RON chain",
    );

    // C4a: the seam's persistent handles survive past Load — whole-session
    // handle persistence (GTW-533), the substrate the live redrives rebuild from.
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
            .get_resource::<HotRonHandle<GdtfThemeSpec>>()
            .is_some(),
        "the theme HotRonHandle must persist past Load",
    );
    assert!(
        world.get_resource::<HotRonHandle<TileRoles>>().is_some(),
        "the tile-roles HotRonHandle must persist past Load",
    );
}

/// AC-4 (the ADR-0003 pin): with an EMPTY asset root every load reaches
/// `Failed`, the fallbacks fire, and `Load` STILL transitions — the editor never
/// hangs. The theme + tile-role chains resolve to their const defaults (the
/// editor-owned zero table: every role at index 0) and the four folder families
/// fail closed to EMPTY registries.
#[test]
fn failed_asset_root_falls_back_and_still_reaches_editing() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the empty asset root must succeed");
    let Ok(dir) = dir else { return };

    let mut app = editor_app_with_asset_root(dir.path());
    // Reaching Editing on an all-failed root IS the no-strand guarantee.
    advance_to_editing(&mut app);

    let world = app.world();
    assert!(
        world.get_resource::<GdtfTheme>().is_some(),
        "a Failed theme must fall back to the const default theme",
    );
    let roles = world.get_resource::<TileRoles>();
    assert_eq!(
        roles.map(|r| r.floor),
        Some(TileIndex::new(0)),
        "a Failed tile-role table must fall back to the editor's zero table (floor)",
    );
    assert_eq!(
        roles.map(|r| r.wall),
        Some(TileIndex::new(0)),
        "a Failed tile-role table must fall back to the editor's zero table (wall)",
    );
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
}
