//! The counter-rule: a DOWNED (not dead) occupant still STOPS the round (AC 5) —
//! only `LifeState::Dead` is transparent to the march.

use bevy::prelude::World;
use gdtf_battle_sim::{LifeState, OccupancyGrid, Wounds};

use super::harness::*;

/// AC #5 — a DOWNED (not Dead) occupant in the ray still STOPS the round: it is hit,
/// NOT passed through (only `LifeState::Dead` is transparent).
#[test]
fn downed_occupant_still_stops_the_round() {
    let mut world = World::new();
    let mode = burst_mode(1); // a single round
    let shooter = spawn_shooter(&mut world, mode);
    // Front: DOWNED (alive, incapacitated) — must still block the round.
    let downed = line_ganger(&mut world, front_cell(), 4, LifeState::Downed);
    // Behind: a live target — it must NOT be reached (the round stops on the downed).
    let behind = line_ganger(&mut world, behind_cell(), 6, LifeState::Alive);

    let mut occupancy = OccupancyGrid::new();
    place_occupant(&mut occupancy, front_cell(), downed);
    place_occupant(&mut occupancy, behind_cell(), behind);

    let behind_wounds_before = world.get::<Wounds>(behind).map_or(0, |w| **w);

    let volley = fire_volley(&mut world, shooter, mode, &occupancy, 0xD09E_D517);

    assert_eq!(volley.reports.len(), 1, "exactly one round fired");
    // The round stopped on the DOWNED occupant (it is hit, not passed through).
    assert!(
        report_struck(&volley, downed),
        "a Downed occupant must STOP the round (only Dead is transparent) — got {:?}",
        volley.reports,
    );
    // The behind target was NOT reached.
    assert!(
        !report_struck(&volley, behind),
        "the round must NOT pass through a Downed occupant to the target behind — got {:?}",
        volley.reports,
    );
    let behind_wounds_after = world.get::<Wounds>(behind).map_or(0, |w| **w);
    assert_eq!(
        behind_wounds_after, behind_wounds_before,
        "the target behind a Downed occupant must take NO effect",
    );
}
