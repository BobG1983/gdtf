use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedShooter};
use gdtf_battle_sim::{
    acts::{
        EndTurnRequested, ReloadRequested, SetAimingRequested, SetFacingRequested,
        SetStanceRequested,
    },
    prelude::{Direction, OccupancyGrid, StanceKind},
};
use gdtf_test_utils::{press_key, press_left, probed};

use super::harness::*;


#[test]
fn end_turn_intent_emits_one_end_turn_requested_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::EndTurn);
    app.update();

    let ends = probed::<EndTurnRequested>(&app);
    assert_eq!(
        ends.len(),
        1,
        "one EndTurnRequested via the end-turn intent, with no selection",
    );
    assert_eq!(
        ends[0], EndTurnRequested,
        "the drained message is the fieldless EndTurnRequested unit value",
    );
}


#[test]
fn no_selection_makes_every_act_a_no_op() {
    let mut app = acts_app();
    add_probes(&mut app);
    let binds = test_keybinds();

    let _ganger = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    app.world_mut().insert_resource(SelectedShooter::cleared());
    let hovered = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    assert!(
        app.world()
            .get_resource::<OccupancyGrid>()
            .and_then(|g| g.occupant(&hovered))
            .is_none(),
        "precondition: the hovered cell is empty (no occupant to select)",
    );

    press_key(&mut app, binds.stance_cycle());
    press_key(&mut app, binds.aim_toggle());
    press_key(&mut app, binds.facing_cycle());
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::Reload);
    press_left(&mut app);
    app.update();

    assert!(
        fires(&app).is_empty(),
        "no FireRequested without a selection"
    );
    assert!(
        probed::<SetStanceRequested>(&app).is_empty(),
        "no SetStanceRequested without a selection",
    );
    assert!(
        probed::<SetAimingRequested>(&app).is_empty(),
        "no SetAimingRequested without a selection",
    );
    assert!(
        probed::<SetFacingRequested>(&app).is_empty(),
        "no SetFacingRequested without a selection",
    );
    assert!(
        probed::<ReloadRequested>(&app).is_empty(),
        "no ReloadRequested without a selection",
    );
}
