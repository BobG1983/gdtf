use bevy::prelude::*;
use gdtf_screenshot::{
    CapturePath, CapturePipelinePlugin, CaptureSystems, SettleFrames, parse_shot_path,
};

use super::systems::{pin_generation_until_shot, request_loading_shot};
use crate::states::{
    BattleScapeState,
    running::game::battlescape::generation::loading_screen::components::LoadingScreenRoot,
};

pub(super) const LOADING_SHOT_ENV: &str = "GDTF_LOADING_SHOT";

const LOADING_SETTLE: SettleFrames = SettleFrames::new(4);

fn loading_shot_path() -> Option<CapturePath> {
    parse_shot_path(std::env::var(LOADING_SHOT_ENV).ok().as_deref())
}

/// Hold the app on the loading screen until the configured PNG lands, then let it move on.
/// Inert unless `GDTF_LOADING_SHOT` names a path.
pub(in crate::states::running::game::battlescape::generation) fn register_loading_capture(
    app: &mut App,
) {
    let Some(path) = loading_shot_path() else {
        return;
    };
    info!("loading-screen capture: ON (dev) -> {}", path.display());
    wire_loading_capture(app, path);
}

pub(super) fn wire_loading_capture(app: &mut App, path: CapturePath) {
    if !app.is_plugin_added::<CapturePipelinePlugin<()>>() {
        app.add_plugins(CapturePipelinePlugin::<()>::new());
    }
    app.insert_resource(path)
        .insert_resource(LOADING_SETTLE)
        .add_systems(
            Update,
            (
                request_loading_shot.run_if(any_with_component::<LoadingScreenRoot>),
                pin_generation_until_shot,
            )
                .chain()
                .before(CaptureSystems)
                .run_if(
                    in_state(BattleScapeState::Generation).and_then(resource_exists::<CapturePath>),
                ),
        );
}
