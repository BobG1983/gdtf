use bevy::prelude::World;
use gdtf_battle_sim::{
    ganger::Wounds,
    prelude::{LifeState, OccupancyGrid},
};

use super::harness::*;

#[test]
fn downed_occupant_still_stops_the_round() {
    let mut world = World::new();
    let mode = burst_mode(1);
    let shooter = spawn_shooter(&mut world, mode);
    let downed = line_ganger(&mut world, front_cell(), 4, LifeState::Downed);
    let behind = line_ganger(&mut world, behind_cell(), 6, LifeState::Alive);

    let mut occupancy = OccupancyGrid::new();
    place_occupant(&mut occupancy, front_cell(), downed);
    place_occupant(&mut occupancy, behind_cell(), behind);

    let behind_wounds_before = world.get::<Wounds>(behind).map_or(0, |w| **w);

    let volley = fire_volley(&mut world, shooter, mode, &occupancy, 0xD09E_D517);

    assert_eq!(volley.reports.len(), 1, "exactly one round fired");
    assert!(
        report_struck(&volley, downed),
        "a Downed occupant must STOP the round (only Dead is transparent) — got {:?}",
        volley.reports,
    );
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
