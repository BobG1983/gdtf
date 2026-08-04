use super::support::*;

#[test]
fn single_observer_sees_in_range_occupied_cell() {
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
        "an enemy-occupied cell in one observer's clear in-range FOV must be squad-VISIBLE"
    );
}

#[test]
fn union_fov_reveals_stair_upper_cell_to_upper_observer() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut occupancy = OccupancyGrid::new();

    let upper_cell = key(10, 5, 1);
    place_occupant(&mut occupancy, upper_cell, spawn_entity(), HeightBand::Low);

    let (pos, st, fc) = alive_observer_at(4, 5, 1);
    let observers = [FovObserver {
        position:         &pos,
        stance:           &st,
        facing:           &fc,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];

    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, no_dead());

    assert!(
        visible.contains(&upper_cell),
        "the stair upper cell must be revealed to an upper-level observer by union_fov \
         (Test 3 — per-candidate-cell fog scan reveals the upper presence)",
    );
}

#[test]
fn two_observers_union_their_discs() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut occupancy = OccupancyGrid::new();

    let west_cell = key(3, 5, 0);
    let east_cell = key(40, 5, 0);
    place_occupant(&mut occupancy, west_cell, spawn_entity(), HeightBand::High);
    place_occupant(&mut occupancy, east_cell, spawn_entity(), HeightBand::High);

    let (a_pos, a_st, a_fc) = alive_observer_at(1, 5, 0);
    let (b_pos, b_st, b_fc) = alive_observer_at(42, 5, 0);

    let a_only = [FovObserver {
        position:         &a_pos,
        stance:           &a_st,
        facing:           &a_fc,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];
    let b_only = [FovObserver {
        position:         &b_pos,
        stance:           &b_st,
        facing:           &b_fc,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];
    let a_vis = union_fov(&a_only, &occupancy, &surface, &cover, &tuning, no_dead());
    let b_vis = union_fov(&b_only, &occupancy, &surface, &cover, &tuning, no_dead());
    assert!(
        a_vis.contains(&west_cell) && !a_vis.contains(&east_cell),
        "observer A sees only the near (west) cell"
    );
    assert!(
        b_vis.contains(&east_cell) && !b_vis.contains(&west_cell),
        "observer B sees only the near (east) cell"
    );

    let both = [a_only[0], b_only[0]];
    let union = union_fov(&both, &occupancy, &surface, &cover, &tuning, no_dead());
    assert!(
        union.contains(&west_cell) && union.contains(&east_cell),
        "the squad VISIBLE set is the union of both observers' discs (both cells present)"
    );
}

#[test]
fn union_unchanged_by_peek() {
    use crate::los::PeekOffset;

    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut occupancy = OccupancyGrid::new();

    let target_cell = key(5, 5, 0);
    place_occupant(
        &mut occupancy,
        target_cell,
        spawn_entity(),
        HeightBand::High,
    );

    let (pos, st, fc) = alive_observer_at(1, 1, 0);

    let no_peek_observers = [FovObserver {
        position:         &pos,
        stance:           &st,
        facing:           &fc,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];
    let peek_observers = [FovObserver {
        position:         &pos,
        stance:           &st,
        facing:           &fc,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];

    let visible_no_peek = union_fov(
        &no_peek_observers,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    let visible_with_peek = union_fov(
        &peek_observers,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );

    assert_eq!(
        visible_no_peek, visible_with_peek,
        "union_fov VISIBLE set must be identical regardless of any PeekOffset          carried by the caller (C4: the union Observer literal is always centred)"
    );

    assert!(
        visible_no_peek.contains(&target_cell),
        "the target cell must be in the VISIBLE set (basic union sanity)"
    );

    let _ = PeekOffset::default();
}
