use super::support::*;

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
    assert_eq!(
        shot.kind,
        ShotKind::Ganger(target),
        "the in-line shot struck the target ganger",
    );
    assert!(
        shot.report.is_some(),
        "a fired round must carry its parallel HitReport (Some), got None: {shot:?}",
    );
    let Some(report) = shot.report.clone() else {
        return;
    };
    assert_eq!(
        report.kind, shot.kind,
        "the report's kind must equal the round's kind (same resolved round, no recompute)",
    );
    assert!(
        matches!(report.verdict, HitVerdict::Ganger(_)),
        "a ganger hit must carry the ganger wound verdict: {report:?}",
    );
}

#[test]
fn clean_miss_carries_some_report_with_no_applied_damage() {
    let mut app = headless_app();
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
    assert!(
        !matches!(shot.kind, ShotKind::Ganger(_)),
        "with no occupant, the round must NOT strike a ganger: {:?}",
        shot.kind,
    );
    assert!(
        shot.report.is_some(),
        "even a clean miss carries its parallel HitReport (Some), got None: {shot:?}",
    );
    let Some(report) = shot.report.clone() else {
        return;
    };
    assert_eq!(
        report.kind, shot.kind,
        "the miss report's kind must equal the round's non-ganger kind (no recompute)",
    );
    assert!(
        !matches!(report.verdict, HitVerdict::Ganger(_)),
        "a non-ganger (miss) round applies no ganger damage: {report:?}",
    );
}

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
