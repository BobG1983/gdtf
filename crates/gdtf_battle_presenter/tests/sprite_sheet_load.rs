//! Sprite sheets: all SheetRole PNGs load from the sprites folder under a real AssetServer.
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
use gdtf_test_utils::{advance_until_load_state, gpu_adapter_probe};

const LOAD_SAFETY_NET: u32 = 10_000;

static GPU_LOCK: Mutex<()> = Mutex::new(());

fn lock_gpu() -> MutexGuard<'static, ()> {
    GPU_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

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

    app.get_sub_app(RenderApp)?;
    Some(app)
}

fn load_sheet(app: &App, role: SheetRole) -> Handle<Image> {
    app.world()
        .resource::<AssetServer>()
        .load(role.asset_path())
}

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

#[test]
fn all_sheet_role_pngs_load_from_sprites_folder() {
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP: no usable GPU adapter in this environment — sprite-sheet load proof not run",
        );
        return;
    }

    let _gpu = lock_gpu();

    let Some(mut app) = build_render_app() else {
        eprintln!(
            "NOTE: all_sheet_role_pngs_load_from_sprites_folder SKIPPED — no GPU adapter present",
        );
        return;
    };

    let terrain = load_sheet(&app, SheetRole::Terrain);
    let characters = load_sheet(&app, SheetRole::Characters);
    let effects = load_sheet(&app, SheetRole::Effects);
    let portraits = load_sheet(&app, SheetRole::Portraits);


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
