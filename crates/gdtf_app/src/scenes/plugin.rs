use bevy::prelude::*;

use crate::scenes::*;

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
        .add_plugins(RunningScenePlugin)
        .add_plugins(TeardownScenePlugin)
}
