//! GTW-290/306 — the per-round `ShotFired` emission: geometry, weapon damage
//! type, burst round count, and the fail-closed empty volley.

use super::support::*;

// GTW-290 — a single in-arc shot emits ONE ShotFired sourced from the round's
// already-computed ShotOutcome: the shooter ref, a muzzle at the shooter's cell, an
// impact at the target cell, and a Ganger kind. Geometry is asserted by RELATION (the
// muzzle/impact cells), not by pinned f32 magnitudes.
#[test]
fn fire_dispatch_emits_one_shot_fired_per_round_with_geometry() {
    // The fire_scenario shooter sits at (2,5) facing East; the HIGH-band target is in
    // line at (8,5,0) — an in-arc, one-round (single_mode shots=1) shot.
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
    // The muzzle is the shooter's 3D fire origin — it floors to the shooter's own cell
    // (2,5). pos_to_cell is the sim-unit floor (no brittle f32 cast), a unit-kind relation.
    let (muzzle_cell, _muzzle_level) = crate::metric::pos_to_cell(shot.muzzle);
    assert_eq!(
        muzzle_cell,
        Cell::new(2, 5),
        "the muzzle sits at the shooter's cell (sourced from ShotOutcome.muzzle)",
    );
    // The impact (cell, level) is the resolved outcome cell — the in-line target cell.
    assert_eq!(
        (shot.impact_cell, shot.impact_level),
        (Cell::new(8, 5), Level::new(0)),
        "the impact cell is the resolved outcome cell (the in-line target)",
    );
    // The trajectory is a non-degenerate sim-unit direction (the cone draw's vector).
    assert!(
        shot.trajectory.vec().length_squared() > 0.0,
        "the trajectory is the round's sampled non-zero unit direction",
    );
}

// GTW-306 — the emitted ShotFired carries the FIRING WEAPON'S DamageType, sourced from
// the shooter's own DamageType component at fire time (pure exposure, no fire-result
// change). To prove the value FLOWS FROM the live component (not a hardcoded constant or a
// default), the shooter's weapon DamageType is OVERRIDDEN to a non-default node (Plasma)
// before the shot, then the emitted message is asserted to carry exactly that node.
#[test]
fn fire_dispatch_shot_fired_carries_the_weapon_damage_type() {
    let (mut app, shooter, _target) = fire_scenario();
    // Override the wielded weapon's damage type to a node distinct from the fixture's
    // default (Kinetic) — so a passing assert can only mean the type was read off the
    // weapon ENTITY's component (GTW-323 slice 2: `ganger → Wields → the weapon
    // entity`), not a constant baked into the emit. Resolve the weapon entity through
    // the relationship, then override its `DamageType`.
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

// GTW-290 — a BURST volley emits one ShotFired PER ROUND (so the presenter draws a
// tracer per round). A 3-shot mode over a 10-round magazine fires 3 rounds → 3 messages.
#[test]
fn fire_dispatch_emits_one_shot_fired_per_burst_round() {
    let (mut app, shooter, _target) = fire_scenario();
    // A 3-round burst (the spawn_shooter magazine loads 10, so all 3 fire).
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

// GTW-290 — a fail-closed fire (a DOWNED shooter never fires) emits NO ShotFired: the
// empty volley has no rounds, so the per-round emit loop writes nothing.
#[test]
fn fail_closed_fire_emits_no_shot_fired() {
    let (mut app, shooter, _target) = fire_scenario();
    // Down the shooter so can_fire fails inside fire() → empty volley, no rounds.
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
