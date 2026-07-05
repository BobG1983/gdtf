//! GTW-537 — the suppressed-mover movement gate: toward reject, exposed reject,
//! covered retreat, and the unsuppressed identity.

use super::support::*;

// GTW-537 (C1) — a SUPPRESSED mover's step TOWARD the suppressor (a cell NOT strictly farther)
// is a HARD reject: MoveRejected{Suppressed}, NO MovementOccurred, and the mover does not move.
// Suppressor at (20,10), mover at (10,10) [Chebyshev 10]; dest (11,10) [Chebyshev 9 < 10] fails
// the "strictly farther" clause. The dest is reachable + affordable, so ONLY the suppression
// gate can reject it — pin-discriminating against the pre-GTW-537 dispatch (which would accept).
#[test]
fn suppressed_move_toward_suppressor_is_rejected() {
    let mut app = headless_app();
    let start = CellLevel::new(Cell::new(10, 10), Level::new(0));
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    suppress_from(&mut app, actor, 20, 10);
    // Even WITH cover behind the toward-dest, the distance clause alone rejects it.
    author_cover(&mut app, 12, 10);

    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0)); // Chebyshev 9 — closer, illegal
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    let rejects = drain_rejects(&mut app);
    assert!(
        rejects
            .iter()
            .any(|r| r.actor == actor && r.reason == MoveRejection::Suppressed),
        "a suppressed mover stepping toward the suppressor must be rejected Suppressed: {rejects:?}",
    );
    assert!(
        drain_movements(&mut app).is_empty(),
        "a rejected suppressed move takes NO step — it announces no MovementOccurred",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(start)),
        "a rejected suppressed move leaves the mover exactly where it was (no partial step)",
    );
}

// GTW-537 (C2) — a SUPPRESSED mover's step to an EXPOSED cell (strictly farther, but the cell
// one step toward the suppressor has NO cover) is a HARD reject on the behind-cover clause:
// MoveRejected{Suppressed}, NO MovementOccurred, no move. Suppressor (20,10), mover (10,10);
// dest (9,10) is farther [Chebyshev 11 > 10] but the toward-cell (10,10) has no registered
// cover, so clause (b) fails. Pin-discriminating: were the behind-cover clause dropped, the
// farther dest would be accepted (a step + a MovementOccurred).
#[test]
fn suppressed_move_to_exposed_farther_cell_is_rejected() {
    let mut app = headless_app();
    let start = CellLevel::new(Cell::new(10, 10), Level::new(0));
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    suppress_from(&mut app, actor, 20, 10);
    // NO cover authored anywhere — the farther dest is exposed.

    let dest = CellLevel::new(Cell::new(9, 10), Level::new(0)); // Chebyshev 11 — farther, exposed
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    let rejects = drain_rejects(&mut app);
    assert!(
        rejects
            .iter()
            .any(|r| r.actor == actor && r.reason == MoveRejection::Suppressed),
        "a suppressed mover to a farther-but-exposed cell must be rejected Suppressed: {rejects:?}",
    );
    assert!(
        drain_movements(&mut app).is_empty(),
        "a rejected suppressed move takes NO step — it announces no MovementOccurred",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(start)),
        "a rejected suppressed move leaves the mover exactly where it was (no partial step)",
    );
}

// GTW-537 (C3) — a SUPPRESSED mover's step to a cell that is BOTH strictly farther AND behind
// cover relative to the suppressor is ACCEPTED: the mover steps (Position changes) and a
// MovementOccurred is announced, with NO MoveRejected. Suppressor (20,10), mover (10,10); dest
// (9,10) is farther [11 > 10], and the toward-cell (10,10) holds registered cover, so BOTH
// clauses pass. This is the "retreat behind cover" the pinned unit is allowed.
#[test]
fn suppressed_move_farther_behind_cover_is_accepted() {
    let mut app = headless_app();
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    suppress_from(&mut app, actor, 20, 10);
    // Cover at (10,10): the cell one step from dest (9,10) toward the suppressor (20,10).
    author_cover(&mut app, 10, 10);

    let dest = CellLevel::new(Cell::new(9, 10), Level::new(0)); // farther AND behind cover
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    assert!(
        drain_rejects(&mut app).is_empty(),
        "a legal suppressed retreat (farther + behind cover) must NOT be rejected",
    );
    let movements = drain_movements(&mut app);
    assert!(
        movements
            .iter()
            .any(|m| m.actor == actor && m.to == Cell::new(9, 10)),
        "a legal suppressed retreat must step the mover and announce a MovementOccurred: {movements:?}",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(dest)),
        "a legal suppressed retreat steps the mover to the destination",
    );
}

// GTW-537 (C4, regression) — an UNSUPPRESSED mover with the IDENTICAL toward-the-suppressor
// geometry is UNAFFECTED: the same (10,10) -> (11,10) step that a suppressed mover is rejected
// for is ACCEPTED (no Suppressed component ⇒ the gate is skipped, the identity path). Pins that
// the gate is scoped to suppressed movers ONLY — it does not silently constrain normal movement.
#[test]
fn unsuppressed_move_with_same_geometry_is_unaffected() {
    let mut app = headless_app();
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    // NO Suppressed component on the mover — the identity path.

    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0)); // toward "would-be" suppressor
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    assert!(
        drain_rejects(&mut app).is_empty(),
        "an unsuppressed mover is never subject to the suppression gate — no MoveRejected",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(dest)),
        "an unsuppressed mover steps freely to the destination (identity path)",
    );
}
