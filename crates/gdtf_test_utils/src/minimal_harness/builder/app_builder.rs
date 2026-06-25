//! The `MinimalPlugins` type-state app builder and its phase markers.

use core::marker::PhantomData;

use bevy::{
    MinimalPlugins, app::App, asset::AssetPlugin, scene::ScenePlugin, state::state::NextState,
    time::TimeUpdateStrategy,
};
use gdtf_app::test_support::{self, AppState};

/// Type-state marker: the builder has **not** yet been given an initial
/// [`AppState`]. In this phase the transition methods
/// ([`starting_in`](GdtfTestAppBuilder::starting_in),
/// [`default_start`](GdtfTestAppBuilder::default_start)) are available but
/// [`build`](GdtfTestAppBuilder::build) is not.
pub struct NoState;

/// Type-state marker: an initial [`AppState`] has been chosen, so
/// [`build`](GdtfTestAppBuilder::build) is now available.
pub struct WithState;

/// A type-state builder for a headless GDTF [`App`].
///
/// The `Phase` type parameter ([`NoState`] then [`WithState`]) tracks whether an
/// initial state has been selected, which gates the API so that `build()` cannot
/// be called before an initial [`AppState`] is chosen. The `PhantomData` uses a
/// `fn() -> Phase` so the unused type parameter is covariant and carries no drop
/// or auto-trait obligations.
pub struct GdtfTestAppBuilder<Phase> {
    /// The headless app under construction.
    app:    App,
    /// Tracks the builder phase at the type level without storing a `Phase`.
    _phase: PhantomData<fn() -> Phase>,
}

impl GdtfTestAppBuilder<NoState> {
    /// Builds a fresh headless app with the full GDTF state stack registered.
    ///
    /// Composes, in order:
    /// 1. [`MinimalPlugins`] — the headless core (no windowing/rendering).
    /// 2. [`TimeUpdateStrategy::FixedTimesteps`]`(1)` — exactly one `FixedUpdate`
    ///    per `App::update()`, making transition timing deterministic.
    /// 3. [`gdtf_app::test_support::register_headless`] — installs `StatesPlugin`
    ///    (which `MinimalPlugins` omits, and `init_state` requires), then the real
    ///    `AppState` plus its sub-states (parent-before-child) and `ScenesPlugin`.
    ///
    /// `register_headless` owns the `StatesPlugin` registration, so the builder
    /// does **not** add it separately — a Bevy plugin may only be added once, and
    /// a duplicate add panics (unlike the idempotent `init_state`/`add_sub_state`).
    ///
    /// The app starts in the default [`AppState`] (`Init`); pick the initial
    /// state with [`starting_in`](Self::starting_in) or keep the default with
    /// [`default_start`](Self::default_start) to reach [`WithState`].
    #[must_use]
    pub fn new() -> Self {
        Self::build_core(false)
    }

    /// Like [`new`](Self::new), but ALSO adds the Bevy SCENE/ASSET infrastructure
    /// (`AssetPlugin` + `ScenePlugin`) the GTW-322 `bsn!` widget builders need —
    /// added BEFORE the GDTF state stack so the `Load` scene plugin sees the
    /// `AssetServer` and registers its RON loaders.
    ///
    /// The `gdtf_ui` widget builders ([`spawn_panel`](gdtf_ui::spawn_panel),
    /// [`spawn_button`](gdtf_ui::spawn_button), etc.) spawn via
    /// `Commands::spawn_scene` (a `bsn!` [`Scene`](bevy::scene::Scene)); the
    /// deferred scene application reads the `AssetServer` + `Assets<ScenePatch>`
    /// resources and panics without them. Any harness that drives a UI panel (menu,
    /// action bar, status / weapon / inspect panel) must use this constructor.
    ///
    /// ORDERING is load-bearing: the `AssetServer` exists when
    /// [`register_headless`](gdtf_app::test_support::register_headless)'s
    /// `ScenesPlugin` builds, so `LoadScenePlugin::build` (which guards its
    /// `init_ron_asset` calls on `AssetServer` presence) registers the RON loader
    /// asset types. So a `default_start` walk through `Load` does not panic on an
    /// uninitialized asset type — the asset loads simply never complete in a
    /// headless walk, and the pre-seeded `Load`-gate resources still drive the
    /// transition. This constructor is **separate** from [`new`](Self::new) so the
    /// `Load`-tier-(a) tests, which deliberately run WITHOUT an `AssetServer` (and
    /// assert the kick-off no-ops), keep using [`new`](Self::new).
    #[must_use]
    pub fn new_with_scene_support() -> Self {
        Self::build_core(true)
    }

    /// Shared construction for [`new`](Self::new) /
    /// [`new_with_scene_support`](Self::new_with_scene_support).
    ///
    /// When `scene_support` is set, `AssetPlugin` + `ScenePlugin` are added FIRST,
    /// so the `AssetServer` is present when the GDTF state stack (and its `Load`
    /// scene plugin's RON-loader registration) builds — see
    /// [`new_with_scene_support`](Self::new_with_scene_support).
    fn build_core(scene_support: bool) -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        if scene_support {
            app.add_plugins((AssetPlugin::default(), ScenePlugin));
        }
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        test_support::register_headless(&mut app);
        Self {
            app,
            _phase: PhantomData,
        }
    }

    /// Selects the initial [`AppState`] the app enters on its first update.
    ///
    /// Queues the transition via [`NextState`]; because `NextState::set` is
    /// buffered and applied during the `StateTransition` schedule on the next
    /// `App::update()`, the app enters `state` (and computes the corresponding
    /// sub-state) on the first call to `update()` after [`build`](Self::build).
    #[must_use]
    pub fn starting_in(mut self, state: AppState) -> GdtfTestAppBuilder<WithState> {
        self.app
            .world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(state);
        GdtfTestAppBuilder {
            app:    self.app,
            _phase: PhantomData,
        }
    }

    /// Keeps the default initial [`AppState`] (`Init`) and advances to
    /// [`WithState`] so the app can be [`build`](Self::build)-ed.
    #[must_use]
    pub fn default_start(self) -> GdtfTestAppBuilder<WithState> {
        GdtfTestAppBuilder {
            app:    self.app,
            _phase: PhantomData,
        }
    }
}

impl Default for GdtfTestAppBuilder<NoState> {
    fn default() -> Self {
        Self::new()
    }
}

impl GdtfTestAppBuilder<WithState> {
    /// Consumes the builder and returns the configured headless [`App`].
    ///
    /// This method exists **only** in the [`WithState`] phase, so it is a compile
    /// error to call `build()` before an initial state has been chosen via
    /// [`starting_in`](GdtfTestAppBuilder::starting_in) or
    /// [`default_start`](GdtfTestAppBuilder::default_start).
    pub fn build(self) -> App {
        self.app
    }
}
