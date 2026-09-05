//! State, scene, and UI registration for headless test apps.

use bevy::{
    app::App,
    asset::AssetServer,
    ecs::system::{Commands, Res},
    input::InputPlugin,
    state::{
        app::{AppExtStates, StatesPlugin},
        state::State,
    },
};
use gdtf_battle_sim::tuning::GangerStatTuning;

use super::markers::UiPlugin;
use crate::states::{AppState, ScenesPlugin, seed_load_fallbacks};

/// Current [`AppState`] on the app.
#[must_use]
pub fn app_state(app: &App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

/// Whether load has finished and the app is in Intro or Running.
#[must_use]
pub fn load_released(app: &App) -> bool {
    matches!(app_state(app), AppState::Intro | AppState::Running)
}

/// Insert load-gate resources when no asset server is present (headless tests).
pub fn seed_load_gate(asset_server: Option<Res<AssetServer>>, mut commands: Commands) {
    if asset_server.is_none() {
        commands.insert_resource(GangerStatTuning::default());
    }
    seed_load_fallbacks(asset_server, commands);
}

/// Register states, scenes, and UI for a headless test app.
pub fn register_headless(app: &mut App) {
    app.add_plugins(StatesPlugin);
    app.init_state::<AppState>();
    app.add_plugins(InputPlugin);
    app.add_plugins(ScenesPlugin);
    app.add_plugins(UiPlugin);
}

/// Register scenes and UI on an app that already has a default plugin stack.
pub fn register_scenes_with_default_plugins(app: &mut App) {
    app.init_state::<AppState>();
    app.add_plugins(ScenesPlugin);
    app.add_plugins(UiPlugin);
}
