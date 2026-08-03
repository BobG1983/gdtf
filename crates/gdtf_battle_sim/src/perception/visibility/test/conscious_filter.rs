use super::support::*;

#[test]
fn only_conscious_observers_contribute() {
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

    let downed = [FovObserver {
        position:         &pos,
        stance:           &st,
        facing:           &fc,
        life:             LifeState::Downed,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];
    let visible_downed = union_fov(&downed, &occupancy, &surface, &cover, &tuning, no_dead());
    assert!(
        !visible_downed.contains(&target_cell),
        "a Downed observer contributes nothing to the squad union"
    );

    let alive = [FovObserver {
        position:         &pos,
        stance:           &st,
        facing:           &fc,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];
    let visible_alive = union_fov(&alive, &occupancy, &surface, &cover, &tuning, no_dead());
    assert!(
        visible_alive.contains(&target_cell),
        "the same observer Alive reveals the cell (the Downed case failed on the conscious gate)"
    );
}

#[test]
fn mixed_observers_only_alive_reveals() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut occupancy = OccupancyGrid::new();

    let downed_side = key(3, 5, 0);
    let alive_side = key(40, 5, 0);
    place_occupant(
        &mut occupancy,
        downed_side,
        spawn_entity(),
        HeightBand::High,
    );
    place_occupant(&mut occupancy, alive_side, spawn_entity(), HeightBand::High);

    let (d_pos, d_st, d_fc) = alive_observer_at(1, 5, 0);
    let (x_pos, x_st, x_fc) = alive_observer_at(2, 5, 0);
    let (a_pos, a_st, a_fc) = alive_observer_at(42, 5, 0);

    let observers = [
        FovObserver {
            position:         &d_pos,
            stance:           &d_st,
            facing:           &d_fc,
            life:             LifeState::Downed,
            stair_eye_offset: StairEyeOffset::new(0.0),
        },
        FovObserver {
            position:         &x_pos,
            stance:           &x_st,
            facing:           &x_fc,
            life:             LifeState::Dead,
            stair_eye_offset: StairEyeOffset::new(0.0),
        },
        FovObserver {
            position:         &a_pos,
            stance:           &a_st,
            facing:           &a_fc,
            life:             LifeState::Alive,
            stair_eye_offset: StairEyeOffset::new(0.0),
        },
    ];

    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, no_dead());
    assert!(
        visible.contains(&alive_side),
        "the Alive observer reveals its in-range cell"
    );
    assert!(
        !visible.contains(&downed_side),
        "the Downed and Dead observers contribute nothing (their near cell stays unseen)"
    );
}
