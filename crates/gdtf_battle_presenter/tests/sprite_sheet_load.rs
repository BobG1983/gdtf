//! GTW-447 (sprites consolidation): in-engine load-state proof that ALL 4
//! [`SheetRole`] sprite-sheet PNGs resolve to [`LoadState::Loaded`] (NOT `Failed`)
//! at their NEW `sprites/` paths after the GTW-447 folder move.
//!
//! The 6 battle sprite PNGs and the 3 spritedef RON files moved from `assets/tiles/`
//! to `assets/sprites/` (the 4 [`SheetRole`] PNGs are the runtime-loaded ones in
//! scope: Terrain, Characters, Effects, Portraits). The RON files are already covered
//! by `include_str!` compile-time guards + existing parse tests; the PNGs have NO
//! compile-time safety net — only a runtime `AssetServer.load` with the new path.
//! This test provides that runtime proof.
//!
//! The PNG load pipeline in Bevy 0.19 completes CPU-side decoding AND populates
//! [`Assets<Image>`] only when a render backend is present (the image data is a
//! render asset — the `no_renderer` `backends: None` headless config does not
//! advance PNG load state to `Loaded`). This test therefore uses the same real-GPU
//! headless harness as `vertical_link_readback.rs` and `fog_shader_readback.rs`:
//! it builds a GPU-capable `App` (real Metal backend, no window), loads all 4 PNGs
//! via the real `AssetServer`, and drives the app until each reaches
//! `LoadState::Loaded`. If no GPU adapter is present (e.g. a GPU-less CI runner)
//! the test skips gracefully with a logged note rather than failing.
//!
//! A wrong path (e.g. a stale `tiles/` reference) causes the `AssetServer` to
//! report `LoadState::Failed` (file not found) — the [`advance_until_load_state`]
//! assert fires, naming the unresolved sheet. This is the automatable in-engine
//! confirmation the GTW-447 ticket requires.
//!
//! Environment: needs a real GPU adapter (Metal on macOS). Single-threaded so two
//! `App`s never init Metal at once (the `fog_shader_readback.rs` precedent). No
//! adapter → the test skips with a note.
//!
//! NO function here takes `&mut World`/`&World`; every `app.world()` call is in the
//! TEST BODY (the `bevy-traps.md` #7 carve-out — the accepted headless-test idiom).

use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, AssetServer, Handle},
    image::Image,
    render::{RenderApp, RenderPlugin},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::SheetRole;
use gdtf_test_utils::advance_until_load_state;

/// Generous safety-net cap for each PNG reaching `LoadState::Loaded`.
///
/// A safety net against a genuine never-resolve hang, NOT a timing budget. The load
/// keys off the terminal `LoadState` signal, so it is deterministic under parallel
/// `cargo` load (GTW-305). Image decoding + GPU upload typically completes within a
/// few dozen frames; `10_000` absorbs any parallel-load contention.
const LOAD_SAFETY_NET: u32 = 10_000;

/// Serialises the real-GPU tests in this binary (the `fog_shader_readback.rs`
/// precedent).
static GPU_LOCK: Mutex<()> = Mutex::new(());

/// Take the process-wide GPU lock, recovering from a poisoned mutex.
fn lock_gpu() -> MutexGuard<'static, ()> {
    GPU_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The workspace-root `assets/` directory (manifest → up two → `assets`) — the same
/// root [`GdtfLoadTestAppBuilder`](gdtf_test_utils::GdtfLoadTestAppBuilder) uses, so
/// a path that loads here loads in the running app.
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Build a render-capable headless `App` with the workspace asset root, or `None`
/// if no GPU adapter is present.
///
/// Mirrors `vertical_link_readback.rs`'s `build_render_app`:
/// `DefaultPlugins` with the real render backend, no window, pipelined rendering
/// disabled so the app drives synchronously, log/winit/audio disabled for headless.
fn build_render_app() -> Option<App> {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..Default::default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..Default::default()
            })
            .set(AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..Default::default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::audio::AudioPlugin>(),
    );

    app.finish();
    app.cleanup();

    // With pipelined rendering disabled, the presence of a `RenderApp` sub-app
    // proves a GPU adapter exists. Its absence means a GPU-less environment —
    // the test should skip rather than fail.
    app.get_sub_app(RenderApp)?;
    Some(app)
}

/// Load `role`'s PNG via the real `AssetServer` and return its handle.
///
/// Calls [`SheetRole::asset_path`] — the SAME path the runtime
/// `load_topdown_atlases` uses — so a stale path in `asset_path` fails here too.
fn load_sheet(app: &App, role: SheetRole) -> Handle<Image> {
    app.world()
        .resource::<AssetServer>()
        .load(role.asset_path())
}

/// Assert that `handle` reached `LoadState::Loaded`, giving a readable message.
///
/// `LoadState` does not implement `PartialEq`, so the assertion calls `is_loaded()`
/// on the current state after the wait has already terminated. A `Failed` or still-
/// loading state (both NOT `Loaded`) will fail this assert with a diagnostic.
fn assert_loaded(app: &App, handle: &Handle<Image>, sheet_name: &str) {
    let state = app
        .world()
        .resource::<AssetServer>()
        .get_load_state(handle.id());
    assert!(
        state
            .as_ref()
            .is_some_and(bevy::asset::LoadState::is_loaded),
        "{sheet_name} must reach LoadState::Loaded (NOT Failed or Loading) — \
         a Failed means file-not-found (stale path?); got: {state:?}",
    );
}

/// GTW-447 — ALL 4 [`SheetRole`] sprite-sheet PNGs resolve to `LoadState::Loaded`
/// (NOT `Failed`) at their NEW `sprites/` paths after the GTW-447 folder move.
///
/// Driven through the REAL `AssetServer` rooted at the workspace `assets/` dir on a
/// GPU-capable headless app. For each role, it issues
/// `asset_server.load::<Image>(SheetRole::asset_path())`, then polls
/// [`advance_until_load_state`] for the terminal `LoadState::Loaded` signal.
///
/// Pin-discriminating: a stale `tiles/` path causes `LoadState::Failed` (file not
/// found) — the wait's timeout assert fires, naming the unresolved sheet. All 4 roles
/// are asserted independently: a single-role test would not catch the others reverting.
///
/// Skips gracefully (prints a note and returns) when no GPU adapter is available —
/// the test is only valid where rendering is possible, mirroring the
/// `fog_shader_readback.rs` / `vertical_link_readback.rs` skip pattern.
#[test]
fn all_sheet_role_pngs_load_from_sprites_folder() {
    let _gpu = lock_gpu();

    let Some(mut app) = build_render_app() else {
        // No GPU adapter — skip rather than fail (the same pattern as the other
        // readback tests; the proof is valid where a GPU exists).
        eprintln!(
            "NOTE: all_sheet_role_pngs_load_from_sprites_folder SKIPPED — no GPU adapter present",
        );
        return;
    };

    // Load all 4 sheets up-front (the AssetServer accepts loads before updates).
    let terrain = load_sheet(&app, SheetRole::Terrain);
    let characters = load_sheet(&app, SheetRole::Characters);
    let effects = load_sheet(&app, SheetRole::Effects);
    let portraits = load_sheet(&app, SheetRole::Portraits);

    // Wait for each PNG to reach its terminal load state, then assert Loaded.
    // `advance_until_load_state` panics with a diagnostic on timeout —
    // the correct failure mode for a genuine never-resolve.

    advance_until_load_state(&mut app, terrain.id(), |s| s.is_loaded(), LOAD_SAFETY_NET);
    assert_loaded(
        &app,
        &terrain,
        "SheetRole::Terrain at `sprites/alt_tileset_terrain.png`",
    );

    advance_until_load_state(
        &mut app,
        characters.id(),
        |s| s.is_loaded(),
        LOAD_SAFETY_NET,
    );
    assert_loaded(
        &app,
        &characters,
        "SheetRole::Characters at `sprites/alt_tileset_characters.png`",
    );

    advance_until_load_state(&mut app, effects.id(), |s| s.is_loaded(), LOAD_SAFETY_NET);
    assert_loaded(
        &app,
        &effects,
        "SheetRole::Effects at `sprites/alt_tileset_effects.png`",
    );

    advance_until_load_state(&mut app, portraits.id(), |s| s.is_loaded(), LOAD_SAFETY_NET);
    assert_loaded(
        &app,
        &portraits,
        "SheetRole::Portraits at `sprites/alt_tileset_portraits.png`",
    );
}
