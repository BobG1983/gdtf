//! `wait` must read the turn tally the frame counted, not the one it is about to count.

use bevy::ecs::schedule::NodeId;

use crate::{
    schedule_support::{a_system_named, ordered_before, update_schedule},
    socket_support::{TestResult, battle_app_listening},
};

#[test]
fn the_wait_handler_runs_after_the_turn_tally_has_counted_this_frame() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let counter = a_system_named(update, "count_turn_changes")?;
    let waiter = a_system_named(update, "handle_wait")?;
    assert!(
        ordered_before(graph, NodeId::System(counter), waiter),
        "handle_wait must be ordered after count_turn_changes; both sit in the same band after \
         QaCommandSystems::Claim, so without that edge the order is the executor's pick and a \
         wait parked this frame reads a tally from before or after the hand-over at random",
    );
    Ok(())
}
