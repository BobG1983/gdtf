use bevy::prelude::*;
use gdtf_ui::focus_nav::FocusNavSystems;

use crate::states::{RunningState, running::menu::systems::*};

pub(in crate::states) struct MenuScenePlugin;

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

    // GTW-122: map an ENABLED menu button's activation to a RunningState change.
    // Both run only while the menu is active. `focus_activated_actions` is ordered
    // `.after(FocusNavSystems::Bridge)` — the set that *writes* `FocusActivated`
    // (gdtf_ui focus-nav) — so an Enter / gamepad-South activation raised this
    // frame is consumed the same frame, never one frame late (bevy-traps rule 3).
    // `mouse_button_actions` reads `Changed<Interaction>` independently of the
    // focus-nav pipeline, so it needs no ordering relative to it.
    app.add_systems(
        Update,
        (
            mouse_button_actions,
            focus_activated_actions.after(FocusNavSystems::Bridge),
        )
            .run_if(in_state(RunningState::Menu)),
    );
}
