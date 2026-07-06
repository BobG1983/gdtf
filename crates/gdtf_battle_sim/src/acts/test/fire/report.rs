//! GTW-302 — the `HitReport` carriage on `ShotFired` (pure exposure of the
//! volley's already-computed report, never a recompute).

use super::support::*;

// GTW-302 — a LANDED ganger hit carries the round's HitReport on the ShotFired: the
// report is Some, its kind is the SAME ShotKind the message kind carries (proving the
// report rides the SAME resolved round, not a re-derivation), and — because the round
// struck the target ganger — the report's applied-damage block is present with the struck
// part. The fire_scenario in-line target at (8,5) is struck on a single round; the assert
// is value-agnostic on the severity MAGNITUDE (seed/tuning dependent), pinning only the
// STRUCTURE the FCT presenter reads.
#[test]
fn landed_ganger_hit_carries_some_report_matching_the_round() {
    let (mut app, shooter, target) = fire_scenario();

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let shots = drain_shots_fired(&mut app);
    assert_eq!(shots.len(), 1, "a single-shot fire emits one ShotFired");
    let Some(shot) = shots.first() else {
        return;
    };
    // The in-line round struck the target ganger — so the message kind is Ganger(target).
    assert_eq!(
        shot.kind,
        ShotKind::Ganger(target),
        "the in-line shot struck the target ganger",
    );
    // GTW-302: the report is carried (Some) — never dropped.
    assert!(
        shot.report.is_some(),
        "a fired round must carry its parallel HitReport (Some), got None: {shot:?}",
    );
    let Some(report) = shot.report.clone() else {
        return;
    };
    // The report rides the SAME resolved round: its kind equals the message's kind
    // (both copied from the round's ShotOutcome — pure exposure, no recompute).
    assert_eq!(
        report.kind, shot.kind,
        "the report's kind must equal the round's kind (same resolved round, no recompute)",
    );
    // A landed Ganger hit carries the boxed ganger verdict (the FCT reads the severity
    // tier, HP damage, life-after, wear outcome, and struck part from it).
    assert!(
        matches!(report.verdict, HitVerdict::Ganger(_)),
        "a ganger hit must carry the ganger wound verdict: {report:?}",
    );
}

// GTW-302 — a CLEAN MISS (no ganger in the round's path) carries a report that REFLECTS
// the miss: the report is Some (never dropped), its kind is a NON-ganger ShotKind, and its
// applied-damage block is None (a non-ganger outcome wounds nothing — resolve_and_apply's
// kind gate). The shooter fires up an EMPTY column (no occupant placed), so no round can
// strike a ganger.
#[test]
fn clean_miss_carries_some_report_with_no_applied_damage() {
    let mut app = headless_app();
    // An armed shooter at (2,5) facing East, NO target placed anywhere in the grid — so
    // the in-arc round down the empty column strikes ground / a slab / nothing, never a
    // ganger.
    let shooter = spawn_shooter(app.world_mut(), 2, 5, single_mode(0.2, 1), false);

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let shots = drain_shots_fired(&mut app);
    assert_eq!(shots.len(), 1, "a single-shot fire emits one ShotFired");
    let Some(shot) = shots.first() else {
        return;
    };
    // No occupant anywhere → the round never struck a ganger.
    assert!(
        !matches!(shot.kind, ShotKind::Ganger(_)),
        "with no occupant, the round must NOT strike a ganger: {:?}",
        shot.kind,
    );
    // GTW-302: the report is still carried (Some) — a miss is a first-class FCT event.
    assert!(
        shot.report.is_some(),
        "even a clean miss carries its parallel HitReport (Some), got None: {shot:?}",
    );
    let Some(report) = shot.report.clone() else {
        return;
    };
    // The report reflects the miss: same non-ganger kind as the message, and NO ganger
    // wound verdict (resolve_and_apply folds a non-ganger round without one).
    assert_eq!(
        report.kind, shot.kind,
        "the miss report's kind must equal the round's non-ganger kind (no recompute)",
    );
    assert!(
        !matches!(report.verdict, HitVerdict::Ganger(_)),
        "a non-ganger (miss) round applies no ganger damage: {report:?}",
    );
}

// GTW-302 — PURE EXPOSURE, not a recompute: the report carried on the ShotFired equals the
// report a same-seed dispatch over an identical world produces (byte-equal). If any
// recompute (a fresh draw) crept into the message path, two same-seed runs would diverge.
// This pins the "equals the volley's already-computed report" clause via the determinism
// property (AC7) on the REAL dispatch path.
#[test]
fn shot_fired_report_is_deterministic_for_the_same_seed() {
    let dispatched_report = || {
        let (mut app, shooter, _target) = fire_scenario();
        app.world_mut().write_message(FireRequested::new(
            shooter,
            single_mode(0.2, 1),
            Cell::new(8, 5),
            Level::new(0),
        ));
        app.update();
        drain_shots_fired(&mut app)
            .first()
            .and_then(|s| s.report.clone())
    };
    assert_eq!(
        dispatched_report(),
        dispatched_report(),
        "the same BattleSeed must reproduce the SAME ShotFired.report — the report is \
         exposed from the volley, never recomputed on the message path",
    );
}
