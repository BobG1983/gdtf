use super::support::*;

#[test]
fn fire_dispatch_emits_one_shot_fired_per_round_with_geometry() {
    let (mut app, shooter, target) = fire_scenario();

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let shots = drain_shots_fired(&mut app);
    assert_eq!(
        shots.len(),
        1,
        "a single-shot fire emits exactly one ShotFired (one round → one tracer)",
    );
    let Some(shot) = shots.first() else {
        return;
    };
    assert_eq!(
        shot.shooter, shooter,
        "the ShotFired must carry the firing shooter entity",
    );
    assert_eq!(
        shot.kind,
        ShotKind::Ganger(target),
        "the in-line shot struck the target ganger — the kind rides off the ShotOutcome",
    );
    let (muzzle_cell, _muzzle_level) = crate::metric::pos_to_cell(shot.muzzle);
    assert_eq!(
        muzzle_cell,
        Cell::new(2, 5),
        "the muzzle sits at the shooter's cell (sourced from ShotOutcome.muzzle)",
    );
    assert_eq!(
        (shot.impact_cell, shot.impact_level),
        (Cell::new(8, 5), Level::new(0)),
        "the impact cell is the resolved outcome cell (the in-line target)",
    );
    assert!(
        shot.trajectory.vec().length_squared() > 0.0,
        "the trajectory is the round's sampled non-zero unit direction",
    );
}

#[test]
fn fire_dispatch_shot_fired_carries_the_weapon_damage_type() {
    let (mut app, shooter, _target) = fire_scenario();
    let weapon = app.world().get::<Wields>(shooter).and_then(Wields::weapon);
    if let Some(weapon) = weapon
        && let Ok(mut entity) = app.world_mut().get_entity_mut(weapon)
    {
        entity.insert(DamageType::Plasma);
    }

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let shots = drain_shots_fired(&mut app);
    assert_eq!(
        shots.len(),
        1,
        "a single-shot fire emits exactly one ShotFired",
    );
    let Some(shot) = shots.first() else {
        return;
    };
    assert_eq!(
        shot.damage,
        DamageType::Plasma,
        "the ShotFired must carry the firing weapon's DamageType (sourced from the \
         shooter's component at fire time), not a constant: {shot:?}",
    );
}

#[test]
fn fire_dispatch_emits_one_shot_fired_per_burst_round() {
    let (mut app, shooter, _target) = fire_scenario();
    let burst = FireModeSpec::new(
        ModeKind::Burst,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.3),
        ModeShots::new(3),
    );

    app.world_mut().write_message(FireRequested::new(
        shooter,
        burst,
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let shots = drain_shots_fired(&mut app);
    assert_eq!(
        shots.len(),
        3,
        "a 3-round burst emits one ShotFired per round (3 tracers): {shots:?}",
    );
    assert!(
        shots.iter().all(|s| s.shooter == shooter),
        "every burst round's ShotFired carries the same firing shooter",
    );
}

#[test]
fn fail_closed_fire_emits_no_shot_fired() {
    let (mut app, shooter, _target) = fire_scenario();
    if let Ok(mut entity) = app.world_mut().get_entity_mut(shooter) {
        entity.insert(LifeState::Downed);
    }

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let shots = drain_shots_fired(&mut app);
    assert!(
        shots.is_empty(),
        "a fail-closed (downed-shooter) fire resolves no round, so emits no ShotFired",
    );
}
