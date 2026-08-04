//! Seeds the load gate for family load tests.

use bevy::{
    app::App,
    ecs::{resource::Resource, system::RunSystemOnce},
};
use gdtf_app::test_support::seed_load_gate;

pub(crate) fn seed_full_load_gate(app: &mut App) {
    let seeded = app.world_mut().run_system_once(seed_load_gate);
    assert!(
        seeded.is_ok(),
        "seed_load_gate's params (Commands + Option<Res<AssetServer>>) are infallible on any \
         world; got {seeded:?}",
    );
}

pub(crate) fn seed_gate_except<R: Resource>(app: &mut App) {
    seed_full_load_gate(app);
    app.world_mut().remove_resource::<R>();
}
