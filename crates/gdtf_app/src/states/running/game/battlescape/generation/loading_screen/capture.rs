use bevy::prelude::*;
use gdtf_screenshot::{
    CapturePath, CaptureProgress, SettleFrames, parse_shot_path, settle_then_capture,
};

use crate::states::{
    BattleScapeState,
    running::game::battlescape::generation::loading_screen::components::LoadingScreenRoot,
};

const LOADING_SHOT_ENV: &str = "GDTF_LOADING_SHOT";

const LOADING_SETTLE: SettleFrames = SettleFrames::new(4);

#[must_use]
pub(in crate::states::running::game::battlescape::generation) fn loading_shot_path()
-> Option<CapturePath> {
    parse_shot_path(std::env::var(LOADING_SHOT_ENV).ok().as_deref())
}

fn pin_generation_until_shot(
    progress: Res<CaptureProgress>,
    mut next: ResMut<NextState<BattleScapeState>>,
) {
    if progress.is_requested() {
        return;
    }
    next.set(BattleScapeState::Generation);
}

/// releases and the app proceeds `Generation → AnimateIn` into the battle). NOTE: the shared
pub(in crate::states::running::game::battlescape::generation) fn register_loading_capture(
    app: &mut App,
) {
    let Some(path) = loading_shot_path() else {
        return;
    };
    info!("loading-screen capture: ON (dev) -> {}", path.display());
    app.insert_resource(path)
        .insert_resource(LOADING_SETTLE)
        .init_resource::<CaptureProgress>()
        .add_systems(
            Update,
            (
                pin_generation_until_shot,
                settle_then_capture.run_if(any_with_component::<LoadingScreenRoot>),
            )
                .chain()
                .run_if(
                    in_state(BattleScapeState::Generation).and_then(resource_exists::<CapturePath>),
                ),
        );
}

#[cfg(test)]
mod tests {
    use super::LOADING_SHOT_ENV;

                    #[test]
    fn env_var_name_is_the_scene_contract() {
        assert_eq!(LOADING_SHOT_ENV, "GDTF_LOADING_SHOT");
    }
}
