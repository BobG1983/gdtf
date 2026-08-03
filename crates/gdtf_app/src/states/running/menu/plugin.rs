use bevy::prelude::*;
use gdtf_ui::focus_nav::FocusNavSystems;

use crate::states::{
    RunningState,
    running::menu::{StartBattleRequested, apply_start_battle, systems::*},
    scaffold::{SceneLabel, log_scene_enter, log_scene_exit},
};

pub(in crate::states) struct MenuScenePlugin;

impl Plugin for MenuScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Running::Menu");
    app.add_systems(
        OnEnter(RunningState::Menu),
        (log_scene_enter(label), spawn_menu),
    )
    .add_systems(
        OnExit(RunningState::Menu),
        (log_scene_exit(label), clear_nav_map),
    );

    app.add_message::<StartBattleRequested>();
    app.add_systems(
        Update,
        (
            mouse_button_actions,
            focus_activated_actions.after(FocusNavSystems::Bridge),
            apply_start_battle
                .after(mouse_button_actions)
                .after(focus_activated_actions),
        )
            .run_if(in_state(RunningState::Menu)),
    );
}
