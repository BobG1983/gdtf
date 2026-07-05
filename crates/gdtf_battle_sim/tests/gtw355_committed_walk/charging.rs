//! C7(b) — TU accounting: an uninterrupted committed walk charges exactly the `find_path`
//! total, bit-for-bit (the §48 identity).

use gdtf_battle_sim::acts::MoveRequested;

use super::harness::*;

// === C7(b) — an UNINTERRUPTED multi-step walk charges EXACTLY the find_path total
// (the §48 bit-identity, end-to-end). ===

#[test]
fn uninterrupted_walk_charges_exactly_the_find_path_total() {
    let mut app = battle_app();
    drive_setup(&mut app, one_player_situation(20.0));

    let Some(actor) = player_entity(&mut app) else {
        unreachable!("setup spawns exactly one player ganger");
    };
    let Some((start, tu_before)) = pos_and_tu(&app, actor) else {
        unreachable!("the player has a Position and Tu");
    };

    // A multi-cell destination a few tiles east, within view range (so routable) and
    // affordable with the spawn TU.
    let dest = ground(8, 5);
    let Some(expected_total) = plan_total(&app, start, dest) else {
        unreachable!("an open in-sight multi-step route exists");
    };
    assert!(
        expected_total > 0,
        "precondition: a real multi-step route has a positive total",
    );

    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    run_until_walk_ends(&mut app, actor);

    let Some((after, tu_after)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    assert_eq!(
        after, dest,
        "an uninterrupted walk reaches the destination cell",
    );
    let spent = tu_before.saturating_sub(tu_after);
    assert_eq!(
        spent, expected_total,
        "the stepped walk charges EXACTLY the find_path total, bit-for-bit (§48)",
    );
}
