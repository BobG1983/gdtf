//! The shadows `battle.reachable` searches over are promoted before it reads them.

use bevy::ecs::schedule::NodeId;

use crate::{
    schedule_support::{a_system_named, ordered_before, update_schedule},
    socket_support::{TestResult, battle_app_listening},
};

#[test]
fn the_shown_grid_and_fog_are_promoted_before_the_reachable_read_searches_them() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let handler = a_system_named(update, "handle_battle_reachable")?;
    for promote in ["promote_shown_occupancy", "promote_shown_fog"] {
        let shadow = a_system_named(update, promote)?;
        assert!(
            ordered_before(graph, NodeId::System(shadow), handler),
            "`{promote}` must run before handle_battle_reachable, or the read searches a \
             shadow promoted for an earlier frame",
        );
    }
    Ok(())
}
