//! AC for [`union_fov`](crate::visibility::union_fov): only CONSCIOUS player-faction
//! observers contribute — a Downed observer (and, by the caller's own faction filtering,
//! an enemy) reveals nothing (GTW-340 clause 5 / third AC).

use super::support::*;

/// A Downed observer contributes nothing to the union, even with a clear in-range line —
/// only an Alive (conscious) observer reveals the same cell.
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

    // A DOWNED observer at the same spot sees nothing — the union is empty.
    let downed = [FovObserver {
        position: &pos,
        stance:   &st,
        facing:   &fc,
        life:     LifeState::Downed,
    }];
    let visible_downed = union_fov(&downed, &occupancy, &surface, &cover, &tuning, no_dead());
    assert!(
        !visible_downed.contains(&target_cell),
        "a Downed observer contributes nothing to the squad union"
    );

    // The SAME observer Alive DOES reveal the cell — proving the Downed case failed on
    // the conscious gate, not the fixture geometry.
    let alive = [FovObserver {
        position: &pos,
        stance:   &st,
        facing:   &fc,
        life:     LifeState::Alive,
    }];
    let visible_alive = union_fov(&alive, &occupancy, &surface, &cover, &tuning, no_dead());
    assert!(
        visible_alive.contains(&target_cell),
        "the same observer Alive reveals the cell (the Downed case failed on the conscious gate)"
    );
}

/// A mixed observer list (one Downed, one Dead, one Alive) reveals exactly the Alive
/// observer's FOV — the inactive ones are skipped.
#[test]
fn mixed_observers_only_alive_reveals() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut occupancy = OccupancyGrid::new();

    // Two occupied candidate cells: one reachable by the (far west) Downed/Dead pair's
    // discs only if they contributed, one reachable by the Alive observer.
    let downed_side = key(3, 5, 0);
    let alive_side = key(40, 5, 0);
    place_occupant(
        &mut occupancy,
        downed_side,
        spawn_entity(),
        HeightBand::High,
    );
    place_occupant(&mut occupancy, alive_side, spawn_entity(), HeightBand::High);

    // Downed observer next to downed_side, Dead observer next to it too — neither should
    // reveal anything. Alive observer next to alive_side — reveals only alive_side.
    let (d_pos, d_st, d_fc) = alive_observer_at(1, 5, 0);
    let (x_pos, x_st, x_fc) = alive_observer_at(2, 5, 0);
    let (a_pos, a_st, a_fc) = alive_observer_at(42, 5, 0);

    let observers = [
        FovObserver {
            position: &d_pos,
            stance:   &d_st,
            facing:   &d_fc,
            life:     LifeState::Downed,
        },
        FovObserver {
            position: &x_pos,
            stance:   &x_st,
            facing:   &x_fc,
            life:     LifeState::Dead,
        },
        FovObserver {
            position: &a_pos,
            stance:   &a_st,
            facing:   &a_fc,
            life:     LifeState::Alive,
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
