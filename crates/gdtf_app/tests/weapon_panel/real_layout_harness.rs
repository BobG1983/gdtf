//! The REAL `bevy_ui` layout harness (GTW-733) — a headless `DefaultPlugins` app (no GPU, no OS
//! window) with a REAL primary [`Window`] and a REAL loaded font, so `Val::Vw`/`Val::Vh` resolve
//! to real pixels and a weapon name gets a REAL measured glyph width (a [`TextLayoutInfo`]) —
//! neither of which the rest of this suite's `MinimalPlugins`-based `battle_running_app`
//! (`harness.rs`) can produce, since it carries no `UiPlugin`/`TextPlugin` layout pass at all.
//!
//! Reserved for the one test that genuinely needs computed geometry
//! ([`super::layout_geometry`]); every other test in this suite keeps using the faster
//! `battle_running_app` (declared-`Node`-field assertions do not need a real layout pass).

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, AssetServer, Assets},
    audio::AudioPlugin,
    ecs::error::warn,
    gizmos::GizmoPlugin,
    log::LogPlugin,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    text::Font,
    window::{ExitCondition, Window, WindowPlugin, WindowResolution},
    winit::WinitPlugin,
};
use gdtf_app::test_support::{self, AppState, BattleScapeState, LoadedSituation, RunningState};
use gdtf_test_utils::advance_until;
use gdtf_ui::theme::default_theme;

use super::harness::BUDGET;

/// Returns the workspace-root `assets/` directory as an absolute path (the
/// `gdtf_test_utils::GdtfUiTestAppBuilder` precedent): `CARGO_MANIFEST_DIR` under `cargo test` is
/// `crates/gdtf_app`, so the workspace root is two parents up.
fn workspace_assets_root() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
        .to_string_lossy()
        .into_owned()
}

/// The reference window resolution the weapon-cluster geometry constants' doc comments are
/// calibrated against (e.g. `GAP_VH`'s "4px gap at 1280x720 reference window") — using the SAME
/// resolution here means this test's measured pixel geometry matches the numbers those doc
/// comments already reason about.
const REFERENCE_WIDTH: u32 = 1280;
/// See [`REFERENCE_WIDTH`].
const REFERENCE_HEIGHT: u32 = 720;

/// Builds a headless [`App`] with `DefaultPlugins` in the official `no_renderer.rs` headless
/// configuration (`RenderPlugin` with no GPU adapter, `WinitPlugin` disabled — the
/// `GdtfUiTestAppBuilder` recipe), but — unlike that harness — with a REAL primary [`Window`] at
/// [`REFERENCE_WIDTH`]x[`REFERENCE_HEIGHT`] (so `Val::Vw`/`Val::Vh` resolve to real physical
/// pixels instead of the zero a windowless app's camera reports) and the `AssetPlugin` rooted at
/// the real workspace `assets/` (so a font path genuinely resolves to a loadable file). Then wires
/// the real GDTF state stack on top via
/// [`register_scenes_with_default_plugins`](test_support::register_scenes_with_default_plugins) —
/// the same production `ScenesPlugin` + `gdtf_ui::UiPlugin` wiring `GdtfApp` itself runs, so the
/// weapon panel's OWN `OnEnter(BattleRunning)` spawn system builds the REAL panel tree, not a
/// hand-assembled copy.
fn new_real_layout_app() -> App {
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
            // Headless-test noise suppression (the GdtfUiTestAppBuilder precedent): silences the
            // otherwise-harmless headless-render logs and skips systems irrelevant to a UI-layout
            // test (Gizmo / Audio / the terminal Ctrl-C handler).
            .disable::<LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<GizmoPlugin>()
            .disable::<AudioPlugin>()
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(REFERENCE_WIDTH, REFERENCE_HEIGHT),
                    ..default()
                }),
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: workspace_assets_root(),
                ..default()
            }),
    );
    // Bevy 0.19 routes a failed system-param validation to the global error handler (the default
    // panics); this headless config disables the render backend, so a render-provided-param
    // system (e.g. a light-gizmo update) cannot validate. `warn` restores the pre-0.19
    // skip-with-a-log behavior for this harness only (the `GdtfUiTestAppBuilder` precedent) — no
    // production code path changes.
    app.set_error_handler(warn);
    // The `GdtfTestAppBuilder` precedent: exactly one `FixedUpdate` per `App::update()`, so the
    // sim's staged procgen `BattleScapeState::Generation` walk (assemble/fill/emit) advances
    // deterministically frame-by-frame instead of at the mercy of real elapsed wall-clock time
    // between calls (which a headless loop with no real frame pacing can make erratic).
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(1));
    test_support::register_scenes_with_default_plugins(&mut app);
    // Jump straight to `Running` (skipping `Init`/`Load`/`Intro`) — the `GdtfTestAppBuilder`
    // `starting_in(AppState::Running)` precedent. This harness seeds the Load-gate registries
    // itself (`seed_load_gate_registries`) rather than waiting on a real asset-driven Load walk.
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Running);
    app
}

/// Seeds the SAME Load-gate-clearing registries `harness::battle_running_app` seeds, PLUS an
/// explicit EMPTY [`LoadedSituation`] (the `seed_load_fallbacks`/GTW-261 "ganger-free default
/// battle" precedent).
///
/// The empty registries are sufficient regardless of whether a registry ends up populated by this
/// app's OWN (now real, workspace-rooted) `AssetServer` finishing a background folder load: the
/// Load state's gate only checks a registry RESOURCE is present, not its provenance (see
/// `harness::battle_running_app`'s doc comments for the per-registry citations) — and because each
/// resource is inserted here BEFORE any resolve system runs, it also SHADOWS that resolve (the
/// documented `AC3b` seed-shadow invariant: a resolve only runs while its resource is absent).
///
/// The `LoadedSituation` seed is NOT optional the way the others are: unlike the registries, an
/// ABSENT `LoadedSituation` does NOT fail closed — this harness's REAL `AssetServer` genuinely
/// resolves it from the real shipped `content/situations/skirmish.ron` in the background
/// (regardless of `AppState`, since that resolve is not gated to `AppState::Load`), which
/// references REAL gangers (`gang_0`'s `Alex Mercer` / `Kira Vann`) that the empty `GangRegistry`
/// above can never satisfy — a mismatch that fails battle setup outright. Seeding an EXPLICIT
/// empty `LoadedSituation` shadows that real resolve, keeping this harness's battle exactly as
/// ganger-free as `harness::battle_running_app`'s (this test's OWN ganger is spawned directly via
/// `spawn_armed_and_select`, matching every other test in this suite).
fn seed_load_gate_registries(app: &mut App) {
    app.world_mut().insert_resource(LoadedSituation::new(
        gdtf_battle_sim::situation::Situation::default(),
    ));
    app.world_mut()
        .insert_resource(gdtf_battle_sim::tuning::CombatTuning::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::injuries::InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
}

/// Builds the [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) resource with a REAL, loaded font handle
/// (unlike `harness::battle_running_app`'s [`default_theme`], which resolves every font to the
/// null `Handle::<Font>::default()` — harmless there since `MinimalPlugins` never measures text at
/// all). Loads [`GdtfTheme::default_font`](gdtf_ui::theme::GdtfTheme::default_font) (the shipped
/// `fonts/Alegreya-Variable.ttf`) through the app's REAL `AssetServer` and drives `app.update()`
/// (bounded by [`BUDGET`]) until the font asset has actually landed in `Assets<Font>` — only once
/// it has does `bevy_text`'s measure pipeline report a REAL glyph width instead of a zero-size
/// placeholder. Returns `true` once the font is confirmed loaded.
fn insert_real_theme_and_wait_for_font(app: &mut App) -> bool {
    let mut theme = default_theme();
    let asset_server = app.world().resource::<AssetServer>().clone();
    // An OWNED `String` (not a borrow of `theme`) so `AssetServer::load` — which needs a
    // `'static` `AssetPath` — is satisfied without fighting `theme`'s later move into
    // `insert_resource`.
    let font_path: String = (*theme.default_font).clone();
    let font: Handle<Font> = asset_server.load(font_path);
    theme.text.font = font.clone();
    theme.button.font = font.clone();
    app.world_mut().insert_resource(theme);

    advance_until(
        app,
        move |app| app.world().resource::<Assets<Font>>().get(&font).is_some(),
        BUDGET,
    )
}

/// Drives a fresh [`new_real_layout_app`] to `BattleScapeState::BattleRunning` with a REAL loaded
/// font in its [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) — the real-layout mirror of
/// `harness::battle_running_app`. Returns `None` if the walk does not reach a live battle or the
/// font does not load within [`BUDGET`] frames (the caller asserts the `Some`).
pub(crate) fn real_layout_battle_running_app() -> Option<App> {
    let mut app = new_real_layout_app();
    seed_load_gate_registries(&mut app);

    if !insert_real_theme_and_wait_for_font(&mut app) {
        return None;
    }

    let at_menu = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<RunningState>>()
                .map(|state| *state.get())
                == Some(RunningState::Menu)
        },
        BUDGET,
    );
    if !at_menu {
        return None;
    }
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let at_battle = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<BattleScapeState>>()
                .map(|state| *state.get())
                == Some(BattleScapeState::BattleRunning)
        },
        BUDGET,
    );
    if !at_battle {
        return None;
    }
    Some(app)
}
