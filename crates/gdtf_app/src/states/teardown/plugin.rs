use bevy::prelude::*;

use crate::states::{
    AppState,
    scaffold::{
        SceneLabel, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
    teardown::{resources::TeardownComplete, systems::*},
};

// Teardown scene plugin for the GDTF app.
pub(in crate::states) struct TeardownScenePlugin;

impl Plugin for TeardownScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Teardown");
    // The marker insert and `move_on` run on the MAIN frame schedule (`Update`), NOT
    // `FixedUpdate` (GTW-311): the winit runner polls `AppExit` / window state once per
    // frame, so the exit must be driven from a per-frame system — a `FixedUpdate` exit
    // can be missed by the runner's poll, compounding the macOS #23313 hang.
    //
    // They are `.chain()`ed (bevy-traps #3): the scaffold marker insert lands
    // `TeardownComplete` via `Commands` (a deferred 1-tick handoff), then on the NEXT
    // frame `move_on`'s `resource_exists::<TeardownComplete>` gate opens and it exits.
    // Chaining keeps their ordering unambiguous within the shared `Update` schedule.
    // `move_on` itself stays BESPOKE (the dual-exit window-despawn + `AppExit` pair —
    // see its divergence doc-comment).
    app.add_systems(OnEnter(AppState::Teardown), log_scene_enter(label))
        .add_systems(
            Update,
            (
                insert_completion_marker::<TeardownComplete>().run_if(
                    in_state(AppState::Teardown).and_then(not(resource_exists::<TeardownComplete>)),
                ),
                move_on.run_if(
                    in_state(AppState::Teardown).and_then(resource_exists::<TeardownComplete>),
                ),
            )
                .chain(),
        )
        .add_systems(
            OnExit(AppState::Teardown),
            (
                log_scene_exit(label),
                remove_scoped_resource::<TeardownComplete>(),
            ),
        );
}
