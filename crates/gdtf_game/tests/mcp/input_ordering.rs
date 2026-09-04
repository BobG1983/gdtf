//! The two pointer commands must move the pointer before the picker projects it onto a cell.

use bevy::ecs::schedule::NodeId;

use super::{
    schedule_support::{a_system_named, ordered_before, update_schedule},
    socket_support::{TestResult, battle_app_listening},
};

#[test]
fn the_pointer_commands_claim_before_the_picker_recomputes_the_hover() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let picker = a_system_named(update, "pick_hovered_cell")?;
    for pointer in ["handle_input_hover", "claim_input_click_cell"] {
        let claim = a_system_named(update, pointer)?;
        assert!(
            ordered_before(graph, NodeId::System(claim), picker),
            "`{pointer}` must run before pick_hovered_cell, the slot the real mouse systems \
             occupy: the picker projects the cursor onto a cell after the pointer has moved, so \
             without that edge the cell a client reads back is a frame stale",
        );
    }
    Ok(())
}
