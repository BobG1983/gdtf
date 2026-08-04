//! Dense-floor visibility vs sparse authored-or-occupied candidates.
use super::support::*;

#[test]
fn flat_open_floor_reveals_own_cell_and_eight_neighbours() {
    let tuning = CombatTuning {
        view_range: ViewRange::new(5),
        ..CombatTuning::default()
    };
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();

    let (ox, oy, oz) = (10, 10, 0);
    let (pos, st, fc) = alive_observer_at(ox, oy, oz);
    let observers = [FovObserver {
        position:         &pos,
        stance:           &st,
        facing:           &fc,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];

    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, no_dead());

    for dy in -1..=1 {
        for dx in -1..=1 {
            let cell = key(ox + dx, oy + dy, oz);
            assert!(
                visible.contains(&cell),
                "the open floor cell at ({}, {}, {}) is within view_range with clear LOS, so \
                 it MUST be revealed (pre-fix the VISIBLE set was empty)",
                ox + dx,
                oy + dy,
                oz
            );
        }
    }

    assert!(
        !visible.is_empty(),
        "an all-Open floor must yield a NON-empty VISIBLE set around the observer"
    );
}
