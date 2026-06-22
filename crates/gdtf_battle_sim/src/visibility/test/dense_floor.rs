//! The GTW-347 **reproducing test**: on a flat all-Open floor (no walls, no occupants), a
//! conscious player observer's [`union_fov`](crate::visibility::union_fov) VISIBLE set
//! CONTAINS the open floor cells within `view_range` with clear LOS — the observer's own
//! cell and all 8 neighbours.
//!
//! This FAILS on the pre-GTW-347 code (the sparse `authored_or_occupied_cells` candidate
//! scan returns NOTHING for an all-Open grid, so the set is EMPTY) and PASSES after (the
//! dense disc scan reveals the floor the presenter draws). The presenter floors every
//! in-range Open cell, so the rendered terrain layer is the whole active-level grid; the
//! fog must reveal the same dense floor or `present_fog` hides the entire map (the bug).

use super::support::*;

/// On a flat all-Open floor with one conscious observer, the VISIBLE set contains the
/// observer's own cell AND all 8 neighbours (every in-disc open-floor cell with clear LOS).
/// On the pre-fix code this set is EMPTY — the regression pin.
#[test]
fn flat_open_floor_reveals_own_cell_and_eight_neighbours() {
    // A flat, entirely-Open grid: no terrain authored, no occupants placed.
    let tuning = CombatTuning {
        view_range: ViewRange::new(5),
        ..CombatTuning::default()
    };
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();

    // One conscious player observer in open space, with room for all 8 neighbours in-grid.
    let (ox, oy, oz) = (10, 10, 0);
    let (pos, st, fc) = alive_observer_at(ox, oy, oz);
    let observers = [FovObserver {
        position: &pos,
        stance:   &st,
        facing:   &fc,
        life:     LifeState::Alive,
    }];

    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, no_dead());

    // The observer's own cell + its 8 Moore neighbours — all open floor, all within
    // view_range, all with a clear line (nothing blocks on an all-Open grid).
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

    // Belt-and-braces: the set is non-empty (the literal regression symptom was an empty
    // set -> an all-black floor).
    assert!(
        !visible.is_empty(),
        "an all-Open floor must yield a NON-empty VISIBLE set around the observer"
    );
}
