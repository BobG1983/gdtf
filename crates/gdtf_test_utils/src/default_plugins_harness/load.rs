//! Headless **real-asset** harness for driving the `AppState::Load` scene.
//!
//! [`GdtfLoadTestAppBuilder`] builds a `DefaultPlugins` app in the official
//! `no_renderer.rs` headless configuration (no GPU, no window) — so it has a live
//! [`AssetServer`](bevy::asset::AssetServer) — **and** registers the real GDTF
//! state stack + scene plugins (via
//! [`gdtf_app::test_support::register_scenes_with_default_plugins`]). That is the
//! combination the [`Load`](gdtf_app::test_support::AppState::Load) orchestration
//! needs: under `MinimalPlugins` (the
//! [`GdtfTestAppBuilder`](crate::GdtfTestAppBuilder)) there is no `AssetServer`,
//! so the async theme/font load cannot run for real.
//!
//! The asset source root is configurable: [`new`](GdtfLoadTestAppBuilder::new)
//! points it at the workspace-root `assets/` (the same root the running app
//! uses, so the good path loads the shipped `theme/grimdark.ron`), while
//! [`with_asset_root`](GdtfLoadTestAppBuilder::with_asset_root) points it at an
//! arbitrary directory — used by the failure-path test to point at a fixtures
//! directory whose `theme/grimdark.ron` is malformed, so the load reaches
//! `Failed` and the const-fallback path is exercised.
//!
//! Like the sibling builders this is `DefaultPlugins`-based; the doctest stays
//! `ignore`d for the `dynamic_linking` dyld reason documented on
//! [`GdtfTestAppBuilder`](crate::GdtfTestAppBuilder).
//!
//! ```ignore
//! use bevy::state::state::State;
//! use gdtf_app::test_support::AppState;
//! use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};
//!
//! let mut app = GdtfLoadTestAppBuilder::new()
//!     .starting_in(AppState::Load)
//!     .build();
//! // Drive a few frames so the async theme load resolves and Load transitions.
//! advance_until(&mut app, |app| { /* GdtfTheme present, etc. */ true }, 64);
//! ```

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::error::warn,
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    state::state::NextState,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_app::test_support::{self, AppState};

/// Returns the workspace-root `assets/` directory as an absolute path.
///
/// Computed lexically from this crate's manifest dir (`crates/gdtf_test_utils` →
/// up two levels → `assets`), mirroring the production app's asset root so a path
/// that loads here loads in the app.
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// A builder for a headless `DefaultPlugins` GDTF [`App`] that can drive the real
/// [`Load`](gdtf_app::test_support::AppState::Load) async asset orchestration.
///
/// It composes the `no_renderer.rs` plugin set (so there is a live `AssetServer`
/// with no GPU/window), points the asset source root at a configurable directory,
/// and registers the real GDTF state stack + scene plugins. Choose the initial
/// state with [`starting_in`](Self::starting_in) before [`build`](Self::build).
pub struct GdtfLoadTestAppBuilder {
    /// The headless app under construction.
    app: App,
}

impl GdtfLoadTestAppBuilder {
    /// Builds a fresh headless real-asset app rooted at the **workspace**
    /// `assets/` directory (the same root the running app uses).
    #[must_use]
    pub fn new() -> Self {
        Self::with_asset_root(workspace_assets_root())
    }

    /// Builds a fresh headless real-asset app rooted at an **arbitrary** assets
    /// directory.
    ///
    /// Used by the failure-path test to point at a fixtures directory whose
    /// `theme/grimdark.ron` is malformed, so the theme load reaches
    /// [`Failed`](bevy::asset::LoadState::Failed) and the const-fallback path
    /// runs. `root` is handed straight to
    /// [`AssetPlugin::file_path`](bevy::asset::AssetPlugin::file_path).
    #[must_use]
    pub fn with_asset_root(root: PathBuf) -> Self {
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
                // Headless-test noise suppression (GTW-139): see GdtfUiTestAppBuilder
                // — disabling LogPlugin leaves no global tracing subscriber, so the
                // headless ERROR/WARN noise (incl. the deliberate failure-path asset
                // errors this harness exercises) does not print; the other three
                // plugins probe a missing window/RenderApp/audio device unused here.
                // (`.disable` is a PluginGroupBuilder method, so it follows `.set`.)
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
        // Bevy 0.19 made a FAILED system-param validation a hard error routed to the
        // global error handler (the default panics); 0.18 silently SKIPPED such a
        // system. This headless harness disables the render backend, so some
        // `DefaultPlugins` systems whose params are render-provided (notably
        // `bevy_light`'s `update_gizmo_meshes::<LightGizmoConfigGroup>`, which wants
        // `Assets<GizmoAsset>`) cannot validate and would intermittently panic the
        // run. `warn` restores the 0.18 skip-with-a-log behavior — these systems are
        // irrelevant to a headless test. No production code path changes (the GUI app
        // keeps the default panicking handler).
        app.set_error_handler(warn);
        // DefaultPlugins already owns StatesPlugin + InputPlugin, so this adds
        // only the state stack + scene/UI plugins (mirroring GdtfApp).
        test_support::register_scenes_with_default_plugins(&mut app);
        Self { app }
    }

    /// Selects the initial [`AppState`] the app enters on its first update.
    ///
    /// Queues the transition via [`NextState`]; the app enters `state` on the
    /// first `App::update()` after [`build`](Self::build).
    #[must_use]
    pub fn starting_in(mut self, state: AppState) -> Self {
        self.app
            .world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(state);
        self
    }

    /// Consumes the builder and returns the configured headless real-asset
    /// [`App`].
    pub fn build(self) -> App {
        self.app
    }
}

impl Default for GdtfLoadTestAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}
