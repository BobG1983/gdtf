use bevy::prelude::*;

use crate::scenes::{
    init::InitScenePlugin, intro::IntroScenePlugin, load::LoadScenePlugin,
    main_menu::MainMenuScenePlugin, playing::PlayingScenePlugin, teardown::TeardownScenePlugin,
};

/// Plugin for registering scenes in the GDTF app.
pub(crate) struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        add_plugins(app);
    }
}

fn add_plugins(app: &mut App) -> &mut App {
    app.add_plugins(InitScenePlugin)
        .add_plugins(LoadScenePlugin)
        .add_plugins(IntroScenePlugin)
        .add_plugins(MainMenuScenePlugin)
        .add_plugins(PlayingScenePlugin)
        .add_plugins(TeardownScenePlugin)
}
