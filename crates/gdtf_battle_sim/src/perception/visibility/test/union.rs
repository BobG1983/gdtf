//! AC for [`union_fov`](crate::visibility::union_fov): a cell in exactly one observer's
//! FOV is squad-VISIBLE, and the union of two observers' discs is the combined VISIBLE
//! set (GTW-340 clause 5 / first AC).

use super::support::*;

/// A cell occupied by an enemy, in exactly ONE conscious observer's clear, in-range
/// FOV, is squad-VISIBLE.
#[test]
fn single_observer_sees_in_range_occupied_cell() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut occupancy = OccupancyGrid::new();

    // An enemy occupant at (8, 5, 0), standing (HIGH band) — the candidate to reveal.
    let target_cell = key(8, 5, 0);
    place_occupant(
        &mut occupancy,
        target_cell,
        spawn_entity(),
        HeightBand::High,
    );

    // One conscious observer at (2, 5, 0) with a clear line and generous range.
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

/// The squad VISIBLE set is the UNION of two observers' discs — a cell only one of them
/// sees AND a cell only the other sees are BOTH in the result.
#[test]
fn two_observers_union_their_discs() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut occupancy = OccupancyGrid::new();

    // Two occupied candidate cells far apart on the x axis.
    let west_cell = key(3, 5, 0);
    let east_cell = key(40, 5, 0);
    place_occupant(&mut occupancy, west_cell, spawn_entity(), HeightBand::High);
    place_occupant(&mut occupancy, east_cell, spawn_entity(), HeightBand::High);

    // Observer A near the west cell (sees west, NOT the far east cell — out of its disc).
    let (a_pos, a_st, a_fc) = alive_observer_at(1, 5, 0);
    // Observer B near the east cell (sees east, NOT the far west cell).
    let (b_pos, b_st, b_fc) = alive_observer_at(42, 5, 0);

    // First confirm each cell is in exactly ONE observer's solo FOV (so the union is a
    // real union, not both-see-both).
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

    // The squad union of A + B holds BOTH cells.
    let both = [a_only[0], b_only[0]];
    let union = union_fov(&both, &occupancy, &surface, &cover, &tuning, no_dead());
    assert!(
        union.contains(&west_cell) && union.contains(&east_cell),
        "the squad VISIBLE set is the union of both observers' discs (both cells present)"
    );
}
