//! only authored/occupied cells, so an in-disc open floor cell was never a candidate — that
use super::support::*;

#[test]
fn out_of_disc_occupied_cell_is_not_visible() {
    let tuning = CombatTuning {
        view_range: ViewRange::new(5),
        ..CombatTuning::default()
    };
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut occupancy = OccupancyGrid::new();

    let near_cell = key(4, 5, 0);
    let far_cell = key(50, 5, 0);
    place_occupant(&mut occupancy, near_cell, spawn_entity(), HeightBand::High);
    place_occupant(&mut occupancy, far_cell, spawn_entity(), HeightBand::High);

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
        visible.contains(&near_cell),
        "the in-disc occupied cell is visible"
    );
    assert!(
        !visible.contains(&far_cell),
        "the far out-of-disc occupied cell is beyond the disc window, so it is never VISIBLE"
    );
}

#[test]
fn open_floor_cell_in_disc_with_clear_los_is_revealed() {
    let tuning = CombatTuning {
        view_range: ViewRange::new(5),
        ..CombatTuning::default()
    };
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();

    let (pos, st, fc) = alive_observer_at(5, 5, 0);
    let observers = [FovObserver {
        position:         &pos,
        stance:           &st,
        facing:           &fc,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];

    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, no_dead());

    let own_cell = key(5, 5, 0);
    let neighbour = key(6, 5, 0);
    assert!(
        visible.contains(&own_cell),
        "the observer's own open-floor cell is revealed by the dense disc scan"
    );
    assert!(
        visible.contains(&neighbour),
        "an in-disc open-floor cell with clear LOS is revealed (the fix)"
    );
    assert!(
        !visible.is_empty(),
        "the all-Open floor's VISIBLE set is NOT empty (the regression pin)"
    );
}
