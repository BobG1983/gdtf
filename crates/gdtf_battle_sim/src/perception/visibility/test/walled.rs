//! AC for [`union_fov`](crate::visibility::union_fov): a cell behind a wall is UNSEEN
//! despite being in range — the LOS probe blocks it (GTW-340 walled-fixture AC; reuses
//! the [`has_los`](crate::los::has_los) HIGH-wall blocking precedent).

use super::support::*;

/// An enemy-occupied cell that is well inside the disc but behind a HIGH wall is NOT in
/// the VISIBLE set — the union runs the LOS probe per candidate, so geometry occlusion
/// keeps a blocked cell UNSEEN.
#[test]
fn cell_behind_wall_is_unseen_despite_in_range() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let mut occupancy = OccupancyGrid::new();
    let mut cover = CoverLedger::new();

    // A standing enemy occupant at (8, 5, 0), well inside the disc.
    let target_cell = key(8, 5, 0);
    place_occupant(
        &mut occupancy,
        target_cell,
        spawn_entity(),
        HeightBand::High,
    );

    // A HIGH wall strictly between a standing observer at (2, 5, 0) and the target — the
    // reused has_los blocking fixture (a HIGH round/sightline is not strictly higher
    // than a HIGH wall).
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

/// Control for the walled fixture: WITHOUT the wall, the same in-range occupant IS
/// VISIBLE — proving the walled case failed on LOS, not on range or the candidate scan.
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
