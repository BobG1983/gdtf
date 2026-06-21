//! AC for [`union_fov`](crate::visibility::union_fov)'s candidate set: it is the
//! disc-bounded authored/occupied entries ONLY — a far out-of-disc occupied cell is
//! never marched, and an empty-air cell inside the disc is never added (GTW-340
//! clause 5 / 6 / fifth AC).

use super::support::*;

/// An occupied cell OUTSIDE the observer's Chebyshev disc is never marched, so it never
/// enters the VISIBLE set — even with a perfectly clear line.
#[test]
fn out_of_disc_occupied_cell_is_not_visible() {
    // A TIGHT view range so the far cell is genuinely out of the disc.
    let tuning = CombatTuning {
        view_range: ViewRange::new(5),
        ..CombatTuning::default()
    };
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut occupancy = OccupancyGrid::new();

    // One occupant well inside the disc, one far beyond it (clear line to both).
    let near_cell = key(4, 5, 0);
    let far_cell = key(50, 5, 0);
    place_occupant(&mut occupancy, near_cell, spawn_entity(), HeightBand::High);
    place_occupant(&mut occupancy, far_cell, spawn_entity(), HeightBand::High);

    let (pos, st, fc) = alive_observer_at(2, 5, 0);
    let observers = [FovObserver {
        position: &pos,
        stance:   &st,
        facing:   &fc,
        life:     LifeState::Alive,
    }];

    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, no_dead());
    assert!(
        visible.contains(&near_cell),
        "the in-disc occupied cell is visible"
    );
    assert!(
        !visible.contains(&far_cell),
        "the far out-of-disc occupied cell is never marched, so it is never VISIBLE"
    );
}

/// An empty-air `(cell, level)` INSIDE the disc — with no terrain entry and no occupant
/// — is never a candidate, so it is never added to the VISIBLE set (the rendered layer
/// IS the fog mask: empty air has no cell to reveal).
#[test]
fn empty_air_cell_in_disc_is_never_added() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut occupancy = OccupancyGrid::new();

    // ONE authored occupant inside the disc; the rest of the grid is empty air.
    let occupied_cell = key(6, 5, 0);
    place_occupant(
        &mut occupancy,
        occupied_cell,
        spawn_entity(),
        HeightBand::High,
    );

    let (pos, st, fc) = alive_observer_at(5, 5, 0);
    let observers = [FovObserver {
        position: &pos,
        stance:   &st,
        facing:   &fc,
        life:     LifeState::Alive,
    }];

    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, no_dead());

    // The visible set is EXACTLY the one authored/occupied cell — no empty-air cell in
    // the disc (e.g. the cells adjacent to the observer, or the observer's own cell) is
    // present.
    assert!(
        visible.contains(&occupied_cell),
        "the authored/occupied cell in the disc is visible"
    );
    let empty_air_neighbor = key(5, 6, 0);
    assert!(
        !visible.contains(&empty_air_neighbor),
        "an empty-air cell inside the disc has no candidate entry, so it is never added"
    );
    assert_eq!(
        visible.len(),
        1,
        "the candidate set is the authored/occupied content only — no disc air is added"
    );
}
