//! AC for [`union_fov`](crate::visibility::union_fov)'s candidate scan: it is the
//! **dense** Chebyshev disc (GTW-347) — a cell BEYOND the disc (out of `view_range`) is
//! excluded AND an LOS-blocked in-disc cell is excluded, BUT an **open floor cell** in-disc
//! with clear LOS IS revealed (the regression fix: the pre-GTW-347 sparse scan revealed
//! only authored/occupied cells, so an in-disc open floor cell was never a candidate — that
//! premise WAS the bug). The walled/blocked + banded-occupant assertions live in
//! `walled.rs` / `banded_occupant.rs`.

use super::support::*;

/// An occupied cell OUTSIDE the observer's Chebyshev disc is never revealed — even with a
/// perfectly clear line. The dense scan's `x`/`y` window is bounded to `± view_range`, and
/// `can_see` re-applies the disc, so a far cell is excluded.
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
        "the far out-of-disc occupied cell is beyond the disc window, so it is never VISIBLE"
    );
}

/// An **open floor cell** INSIDE the disc with clear LOS IS revealed — the GTW-347 fix. The
/// dense disc scans every in-range `(x, y)` (not just authored/occupied cells), so the open
/// floor the presenter draws is in the VISIBLE set. (Pre-fix, this set was EMPTY: an
/// all-Open grid has NO authored/occupied candidates, so `present_fog` hid the whole map —
/// this assertion pins the regression.)
#[test]
fn open_floor_cell_in_disc_with_clear_los_is_revealed() {
    // An entirely empty (all-Open) grid — no walls, no occupants, no authored content.
    let tuning = CombatTuning {
        view_range: ViewRange::new(5),
        ..CombatTuning::default()
    };
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();

    let (pos, st, fc) = alive_observer_at(5, 5, 0);
    let observers = [FovObserver {
        position: &pos,
        stance:   &st,
        facing:   &fc,
        life:     LifeState::Alive,
    }];

    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, no_dead());

    // The observer's own cell and an in-disc open-floor neighbour are both revealed —
    // exactly the floor the presenter renders. Pre-fix, the set was empty (no candidates).
    let own_cell = key(5, 5, 0);
    let neighbour = key(6, 5, 0);
    assert!(
        visible.contains(&own_cell),
        "the observer's own open-floor cell is revealed by the dense disc scan"
    );
    assert!(
        visible.contains(&neighbour),
        "an in-disc open-floor cell with clear LOS is revealed (the GTW-347 fix)"
    );
    assert!(
        !visible.is_empty(),
        "the all-Open floor's VISIBLE set is NOT empty (the regression pin)"
    );
}
