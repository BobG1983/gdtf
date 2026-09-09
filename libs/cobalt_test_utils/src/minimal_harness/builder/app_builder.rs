//! Typed-builder phases for a `MinimalPlugins` headless app.

use core::marker::PhantomData;

use bevy::{
    MinimalPlugins,
    app::App,
    scene::ScenePlugin,
    state::state::{FreelyMutableState, NextState, States},
    time::TimeUpdateStrategy,
};

use crate::asset_plugin::unwatched_asset_plugin;

/// Builder phase: no starting state set yet.
pub struct NoState;

/// Builder phase: starting state has been chosen.
pub struct WithState;

/// `MinimalPlugins` headless app builder.
pub struct MinimalTestAppBuilder<Phase> {
    app:    App,
    _phase: PhantomData<fn() -> Phase>,
}

impl MinimalTestAppBuilder<NoState> {
    /// `MinimalPlugins` only (no scene support), then `register`.
    #[must_use]
    pub fn new(register: impl FnOnce(&mut App)) -> Self {
        Self::build_core(false, register)
    }

    /// `MinimalPlugins` plus `AssetPlugin` and `ScenePlugin`, then `register`.
    #[must_use]
    pub fn new_with_scene_support(register: impl FnOnce(&mut App)) -> Self {
        Self::build_core(true, register)
    }

    fn build_core(scene_support: bool, register: impl FnOnce(&mut App)) -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        if scene_support {
            app.add_plugins((unwatched_asset_plugin(), ScenePlugin));
        }
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        register(&mut app);
        Self {
            app,
            _phase: PhantomData,
        }
    }

    /// Queue a transition into `state` before the first update.
    #[must_use]
    pub fn starting_in<S: States + FreelyMutableState>(
        mut self,
        state: S,
    ) -> MinimalTestAppBuilder<WithState> {
        self.app
            .world_mut()
            .resource_mut::<NextState<S>>()
            .set(state);
        MinimalTestAppBuilder {
            app:    self.app,
            _phase: PhantomData,
        }
    }

    /// Keep the registered default state and move to the `WithState` phase.
    #[must_use]
    pub fn default_start(self) -> MinimalTestAppBuilder<WithState> {
        MinimalTestAppBuilder {
            app:    self.app,
            _phase: PhantomData,
        }
    }
}

impl MinimalTestAppBuilder<WithState> {
    /// Finish building and return the app.
    pub fn build(self) -> App {
        self.app
    }
}
