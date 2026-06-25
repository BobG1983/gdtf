//! AC for [`union_fov`](crate::visibility::union_fov): a cell in exactly one observer's
//! FOV is squad-VISIBLE, and the union of two observers' discs is the combined VISIBLE
//! set (GTW-340 clause 5 / first AC).
//!
//! GTW-391: the dual-cell stair upper-cell presence is revealed to an upper-level
//! observer by the per-candidate-cell fog scan (the shipped sight consumer).

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

/// GTW-391 Test 3 (fog-reveal): the upper-cell stair presence `(x, y, z+1)` is revealed
/// to a conscious observer on storey z+1 by [`union_fov`]'s per-candidate-cell dense scan.
///
/// `union_fov` probes EVERY `(cell, level)` in the observer's Chebyshev disc (including
/// the upper cell) as its own candidate: `Target.position = upper_cell`, and
/// `can_see` → `has_los` marches from the observer's eye at z+1 to the aim anchor at
/// `upper_cell`. Because the upper cell carries a Low-band occupant, the aim anchor
/// resolves as `Low`, and the march from the observer (same storey z+1) to the upper
/// cell terminates there (`result.at == target_cell == upper`) → `is_clear` → CLEAR.
/// The upper cell is then inserted into the visible set. This is the shipped fog-reveal
/// path for the dual-cell stair presence (Blocker 1 resolution — no `has_los` change).
#[test]
fn union_fov_reveals_stair_upper_cell_to_upper_observer() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut occupancy = OccupancyGrid::new();

    // The stair occupant's upper cell sits at (10, 5, 1) — level 1.
    let upper_cell = key(10, 5, 1);
    // Simulate what register_stair_presence writes: occupant at upper cell with Low band.
    place_occupant(&mut occupancy, upper_cell, spawn_entity(), HeightBand::Low);

    // A conscious observer on level 1 (the same storey as the upper cell), close range.
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
         (GTW-391 Test 3 — per-candidate-cell fog scan reveals the upper presence)",
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
