use bevy::prelude::*;

use crate::{
    scenes::teardown::{resources::TeardownComplete, systems::*},
    states::AppState,
};

// Teardown scene plugin for the GDTF app.
pub(in crate::scenes) struct TeardownScenePlugin;

impl Plugin for TeardownScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    // `teardown_complete` and `move_on` run on the MAIN frame schedule (`Update`), NOT
    // `FixedUpdate` (GTW-311): the winit runner polls `AppExit` / window state once per
    // frame, so the exit must be driven from a per-frame system — a `FixedUpdate` exit
    // can be missed by the runner's poll, compounding the macOS #23313 hang.
    //
    // They are `.chain()`ed (bevy-traps #3): `teardown_complete` inserts
    // `TeardownComplete` via `Commands` (a deferred 1-tick handoff), then on the NEXT
    // frame `move_on`'s `resource_exists::<TeardownComplete>` gate opens and it exits.
    // Chaining keeps their ordering unambiguous within the shared `Update` schedule.
    app.add_systems(OnEnter(AppState::Teardown), print_on_enter)
        .add_systems(
            Update,
            (
                teardown_complete.run_if(
                    in_state(AppState::Teardown).and_then(not(resource_exists::<TeardownComplete>)),
                ),
                move_on.run_if(
                    in_state(AppState::Teardown).and_then(resource_exists::<TeardownComplete>),
                ),
            )
                .chain(),
        )
        .add_systems(OnExit(AppState::Teardown), (print_on_exit, cleanup));
}
