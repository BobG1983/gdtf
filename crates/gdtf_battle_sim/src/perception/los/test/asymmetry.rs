use super::support::*;

#[test]
fn low_sees_tall_but_tall_blocked_by_ground_wall() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    let mut cover = CoverLedger::new();
    cover.insert(key(7, 5, 0), cover_entry(HeightBand::High));

    let low_pos = position(2, 5, 0);
    let low_stance = stance(StanceKind::Prone);
    let low_facing = facing(Direction::East);
    let tall_pos = position(8, 5, 1);
    let tall_stance = stance(StanceKind::Prone);
    let tall_facing = facing(Direction::West);

    let low_observer = Observer {
        position:         &low_pos,
        stance:           &low_stance,
        facing:           &low_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let low_target = Target {
        position: &low_pos,
        stance:   &low_stance,
    };
    let tall_observer = Observer {
        position:         &tall_pos,
        stance:           &tall_stance,
        facing:           &tall_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let tall_target = Target {
        position: &tall_pos,
        stance:   &tall_stance,
    };

    let low_to_tall = has_los(
        &low_observer,
        &tall_target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    let tall_to_low = has_los(
        &tall_observer,
        &low_target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );

    assert!(
        *low_to_tall,
        "the low watcher must SEE the tall target (CLEAR)"
    );
    assert!(
        !*tall_to_low,
        "the tall watcher must be BLOCKED looking back (asymmetry)"
    );
    assert_ne!(
        *low_to_tall, *tall_to_low,
        "the verdict must be asymmetric in the same world"
    );
}
