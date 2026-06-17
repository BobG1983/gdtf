//! AC4 — posture dispatch runs the verbs with their TU semantics.

use super::support::*;

#[test]
fn set_stance_dispatch_changes_stance_and_spends_the_tuning_leaf() {
    let mut app = headless_app();
    let actor = app
        .world_mut()
        .spawn((Stance::new(StanceKind::Standing), Tu::new(60)))
        .id();
    let cost = app
        .world()
        .get_resource::<CombatTuning>()
        .map(|t| *t.stance_change_tu);
    let tu_before = app.world().get::<Tu>(actor).map(|t| **t);

    app.world_mut()
        .write_message(SetStanceRequested::new(actor, StanceKind::Prone));
    app.update();

    let stance_after = app.world().get::<Stance>(actor).map(|s| **s);
    let tu_after = app.world().get::<Tu>(actor).map(|t| **t);
    assert_eq!(
        stance_after,
        Some(StanceKind::Prone),
        "set-stance dispatch must change the stance to the requested value",
    );
    // The TU drop equals exactly the consulted stance_change_tu leaf (a relation to
    // the tuning value, never a pinned magnitude).
    assert!(
        matches!((tu_before, tu_after), (Some(b), Some(a)) if a < b),
        "a real stance change must strictly decrease Tu",
    );
    assert_eq!(
        tu_before.zip(tu_after).map(|(b, a)| b - a),
        cost,
        "the Tu drop must equal exactly the stance_change_tu tuning leaf",
    );
}

#[test]
fn set_stance_dispatch_to_same_stance_is_a_no_op_on_tu() {
    let mut app = headless_app();
    let actor = app
        .world_mut()
        .spawn((Stance::new(StanceKind::Crouching), Tu::new(60)))
        .id();
    app.world_mut()
        .write_message(SetStanceRequested::new(actor, StanceKind::Crouching));
    app.update();
    assert_eq!(
        app.world().get::<Tu>(actor).map(|t| **t),
        Some(60),
        "re-asserting the held stance must not spend any TU (the verb's no-op)",
    );
    assert_eq!(
        app.world().get::<Stance>(actor).map(|s| **s),
        Some(StanceKind::Crouching),
        "the stance is unchanged on a no-op",
    );
}

#[test]
fn set_facing_dispatch_changes_facing_and_spends_the_tuning_leaf() {
    let mut app = headless_app();
    let actor = app
        .world_mut()
        .spawn((Facing::new(Direction::North), Tu::new(50)))
        .id();
    // The per-step turn cost off the shipped tuning, scaled by the short-way step
    // count (North -> East is a 2-step turn) — the expected fully-affordable charge.
    let per_step = app
        .world()
        .get_resource::<CombatTuning>()
        .map(|t| *t.turn_tu);
    let expected_drop = per_step.map(|c| Direction::North.steps_to(Direction::East) * c);
    let tu_before = app.world().get::<Tu>(actor).map(|t| **t);

    app.world_mut()
        .write_message(SetFacingRequested::new(actor, Direction::East));
    app.update();

    assert_eq!(
        app.world().get::<Facing>(actor).map(|f| **f),
        Some(Direction::East),
        "set-facing dispatch must turn to the requested direction",
    );
    let tu_after = app.world().get::<Tu>(actor).map(|t| **t);
    assert!(
        matches!((tu_before, tu_after), (Some(b), Some(a)) if a < b),
        "a real turn must strictly decrease Tu",
    );
    assert_eq!(
        tu_before.zip(tu_after).map(|(b, a)| b - a),
        expected_drop,
        "the Tu drop must equal exactly (short-way steps) * the per-step turn_tu leaf",
    );
}

#[test]
fn set_facing_dispatch_to_same_facing_is_a_no_op_on_tu() {
    let mut app = headless_app();
    let actor = app
        .world_mut()
        .spawn((Facing::new(Direction::SouthWest), Tu::new(50)))
        .id();
    app.world_mut()
        .write_message(SetFacingRequested::new(actor, Direction::SouthWest));
    app.update();
    assert_eq!(
        app.world().get::<Tu>(actor).map(|t| **t),
        Some(50),
        "re-asserting the held facing must not spend any TU (the verb's no-op)",
    );
}

#[test]
fn set_aiming_dispatch_flips_the_flag_and_leaves_tu_unchanged() {
    let mut app = headless_app();
    let actor = app
        .world_mut()
        .spawn((Aiming::new(false), Tu::new(40)))
        .id();
    app.world_mut()
        .write_message(SetAimingRequested::new(actor, AimRequest::new(true)));
    app.update();
    assert_eq!(
        app.world().get::<Aiming>(actor).map(|a| **a),
        Some(true),
        "set-aiming dispatch must flip the aim flag to the requested value",
    );
    // Toggling aim is free — set_aiming spends no TU (posture.rs).
    assert_eq!(
        app.world().get::<Tu>(actor).map(|t| **t),
        Some(40),
        "toggling aim must NOT change Tu (aiming is free)",
    );
}
