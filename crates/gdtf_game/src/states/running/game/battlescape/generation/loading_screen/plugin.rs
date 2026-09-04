use bevy::prelude::*;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::generation::loading_screen::spawn::spawn_loading_screen,
};

pub(in crate::states::running::game::battlescape::generation) struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(BattleScapeState::Generation), spawn_loading_screen);
    }
}
