use bevy::prelude::*;

use crate::{scenes::running::menu::systems::*, states::RunningState};

pub(in crate::scenes) struct MenuScenePlugin;

impl Plugin for MenuScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    // GTW-121: spawn the full themed menu on entry; there is NO auto-advance —
    // a menu button transition is player-driven (wired in GTW-122). On exit, the
    // menu's tree is despawned by its `DespawnOnExit(RunningState::Menu)` markers
    // (state-scoped), and `clear_nav_map` drops the now-stale nav edges.
    app.add_systems(OnEnter(RunningState::Menu), (print_on_enter, spawn_menu))
        .add_systems(OnExit(RunningState::Menu), (print_on_exit, clear_nav_map));
}
