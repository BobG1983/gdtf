use super::support::*;

#[test]
fn co_located_from_eq_to_is_sighted() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();

    let at = position(5, 5, 0);
    let from_stance = stance(StanceKind::Standing);
    let from_facing = facing(Direction::North);
    let to_stance = stance(StanceKind::Crouching);

    let observer = Observer {
        position:         &at,
        stance:           &from_stance,
        facing:           &from_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let target = Target {
        position: &at,
        stance:   &to_stance,
    };

    let sighted = has_los(
        &observer,
        &target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *sighted,
        "a co-located from==to is degenerate → CLEAR, never a panic"
    );
}

#[test]
fn off_grid_never_panics() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();

    let on_grid = position(5, 5, 0);
    let off_grid = position(-10, 200, 0);
    let st = stance(StanceKind::Standing);
    let look = facing(Direction::East);

    let off_observer = Observer {
        position:         &off_grid,
        stance:           &st,
        facing:           &look,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let on_target = Target {
        position: &on_grid,
        stance:   &st,
    };
    let a1 = has_los(
        &off_observer,
        &on_target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    let a2 = has_los(
        &off_observer,
        &on_target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );

    let on_observer = Observer {
        position:         &on_grid,
        stance:           &st,
        facing:           &look,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let off_target = Target {
        position: &off_grid,
        stance:   &st,
    };
    let b1 = has_los(
        &on_observer,
        &off_target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    let b2 = has_los(
        &on_observer,
        &off_target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );

    assert_eq!(
        a1, a2,
        "an off-grid observer must return a defined, stable verdict"
    );
    assert_eq!(
        b1, b2,
        "an off-grid target must return a defined, stable verdict"
    );
}
