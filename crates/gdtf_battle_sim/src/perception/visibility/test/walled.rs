use super::support::*;

#[test]
fn cell_behind_wall_is_unseen_despite_in_range() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let mut occupancy = OccupancyGrid::new();
    let mut cover = CoverLedger::new();

    let target_cell = key(8, 5, 0);
    place_occupant(
        &mut occupancy,
        target_cell,
        spawn_entity(),
        HeightBand::High,
    );

    cover.insert(key(5, 5, 0), cover_entry(HeightBand::High));

    let (pos, st, fc) = alive_observer_at(2, 5, 0);
    let observers = [FovObserver {
        position:         &pos,
        stance:           &st,
        facing:           &fc,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];

    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, no_dead());
    assert!(
        !visible.contains(&target_cell),
        "a cell behind a HIGH wall is UNSEEN despite being in range (the LOS probe blocks it)"
    );
}

#[test]
fn same_cell_visible_without_the_wall() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut occupancy = OccupancyGrid::new();

    let target_cell = key(8, 5, 0);
    place_occupant(
        &mut occupancy,
        target_cell,
        spawn_entity(),
        HeightBand::High,
    );

    let (pos, st, fc) = alive_observer_at(2, 5, 0);
    let observers = [FovObserver {
        position:         &pos,
        stance:           &st,
        facing:           &fc,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];

    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, no_dead());
    assert!(
        visible.contains(&target_cell),
        "without the wall the same in-range occupant is VISIBLE (the walled case failed on LOS)"
    );
}
