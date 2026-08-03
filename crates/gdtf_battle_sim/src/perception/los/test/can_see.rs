use super::support::*;

fn clear_pair() -> (Position, Stance, Facing, Position, Stance) {
    (
        position(2, 5, 0),
        stance(StanceKind::Standing),
        facing(Direction::East),
        position(8, 5, 0),
        stance(StanceKind::Standing),
    )
}

fn clear_pair_chebyshev() -> u16 {
    let (from_pos, _, _, to_pos, _) = clear_pair();
    let dx = (from_pos.x - to_pos.x).unsigned_abs();
    let dy = (from_pos.y - to_pos.y).unsigned_abs();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the test fixture deltas are tiny; this conversion is exact"
    )]
    let max = dx.max(dy) as u16;
    max
}

#[test]
fn at_range_edge_with_clear_los_is_true() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();
    let (from_pos, from_stance, from_facing, to_pos, to_stance) = clear_pair();

    let observer = Observer {
        position:         &from_pos,
        stance:           &from_stance,
        facing:           &from_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let view_range = ViewRange::new(clear_pair_chebyshev());
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Alive,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *verdict,
        "at Chebyshev distance == view_range with clear LOS, can_see must be true (<= is inclusive)"
    );
}

#[test]
fn beyond_range_edge_is_false() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();
    let (from_pos, from_stance, from_facing, to_pos, to_stance) = clear_pair();

    let observer = Observer {
        position:         &from_pos,
        stance:           &from_stance,
        facing:           &from_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let view_range = ViewRange::new(clear_pair_chebyshev() - 1);
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Alive,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*verdict,
        "at Chebyshev distance == view_range + 1, can_see must be false (the disc edge is exclusive beyond range)"
    );
}

#[test]
fn downed_observer_sees_nothing() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();
    let (from_pos, from_stance, from_facing, to_pos, to_stance) = clear_pair();

    let observer = Observer {
        position:         &from_pos,
        stance:           &from_stance,
        facing:           &from_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let view_range = ViewRange::new(clear_pair_chebyshev());
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Downed,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*verdict,
        "a Downed observer must see nothing regardless of range / LOS"
    );
}

#[test]
fn dead_observer_sees_nothing() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();
    let (from_pos, from_stance, from_facing, to_pos, to_stance) = clear_pair();

    let observer = Observer {
        position:         &from_pos,
        stance:           &from_stance,
        facing:           &from_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let view_range = ViewRange::new(clear_pair_chebyshev());
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Dead,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*verdict,
        "a Dead observer must see nothing regardless of range / LOS"
    );
}

#[test]
fn alive_observer_in_range_with_clear_los_is_true() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();
    let (from_pos, from_stance, from_facing, to_pos, to_stance) = clear_pair();

    let observer = Observer {
        position:         &from_pos,
        stance:           &from_stance,
        facing:           &from_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let view_range = ViewRange::new(clear_pair_chebyshev());
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Alive,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *verdict,
        "an Alive observer, in range, with clear LOS must see the target"
    );
}

#[test]
fn blocked_los_inside_range_is_false() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    let mut cover = CoverLedger::new();
    cover.insert(key(5, 5, 0), cover_entry(HeightBand::High));

    let (from_pos, from_stance, from_facing, to_pos, to_stance) = clear_pair();
    let observer = Observer {
        position:         &from_pos,
        stance:           &from_stance,
        facing:           &from_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let view_range = ViewRange::new(clear_pair_chebyshev() + 10);
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Alive,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*verdict,
        "a blocked line of sight inside range must make can_see false"
    );
}

#[test]
fn chebyshev_ignores_level_axis() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();

    let from_pos = position(4, 4, 0);
    let from_stance = stance(StanceKind::Standing);
    let from_facing = facing(Direction::North);
    let to_pos = position(4, 4, 1);
    let to_stance = stance(StanceKind::Standing);

    let observer = Observer {
        position:         &from_pos,
        stance:           &from_stance,
        facing:           &from_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let view_range = ViewRange::new(0);
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Alive,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *verdict,
        "the Chebyshev range disc must ignore the level/z axis (z is the LOS probe's)"
    );
}
