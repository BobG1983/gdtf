//! Typed-builder phases for a `MinimalPlugins` headless app.

use core::marker::PhantomData;

use bevy::{
    MinimalPlugins, app::App, asset::AssetPlugin, scene::ScenePlugin, state::state::NextState,
    time::TimeUpdateStrategy,
};
use gdtf_app::test_support::{self, AppState};

/// Builder phase: no starting state set yet.
pub struct NoState;

/// Builder phase: starting state has been chosen.
pub struct WithState;

/// `MinimalPlugins` headless app builder.
pub struct GdtfTestAppBuilder<Phase> {
    app: App,
    _phase: PhantomData<fn() -> Phase>,
}

impl GdtfTestAppBuilder<NoState> {
    /// `MinimalPlugins` only (no scene support).
    #[must_use]
    pub fn new() -> Self {
        Self::build_core(false)
    }

    /// `MinimalPlugins` plus `AssetPlugin` and `ScenePlugin`.
    #[must_use]
    pub fn new_with_scene_support() -> Self {
        Self::build_core(true)
    }

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

    /// Queue a transition into `state` before the first update.
    #[must_use]
    pub fn starting_in(mut self, state: AppState) -> GdtfTestAppBuilder<WithState> {
        self.app
            .world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(state);
        GdtfTestAppBuilder {
            app: self.app,
            _phase: PhantomData,
        }
    }

    /// Keep the default `AppState` and move to the `WithState` phase.
    #[must_use]
    pub fn default_start(self) -> GdtfTestAppBuilder<WithState> {
        GdtfTestAppBuilder {
            app: self.app,
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
    /// Finish building and return the app.
    pub fn build(self) -> App {
        self.app
    }
}
