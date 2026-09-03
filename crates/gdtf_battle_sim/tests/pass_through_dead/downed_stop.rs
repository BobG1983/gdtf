use bevy::prelude::World;
use gdtf_battle_sim::{
    cover::HeightBand,
    ganger::Hp,
    inflicted_wound::InflictedWounds,
    prelude::{LifeState, OccupancyGrid},
};

use super::harness::*;

#[test]
fn a_round_at_the_floor_hits_and_wounds_a_downed_ganger() {
    let mut world = World::new();
    let mode = burst_mode(1);
    let shooter = spawn_shooter(&mut world, mode);
    let downed = line_ganger(&mut world, front_cell(), 4, LifeState::Downed);

    let mut occupancy = OccupancyGrid::new();
    place_occupant(&mut occupancy, front_cell(), downed, HeightBand::Low);

    let wounds_before = world.get::<InflictedWounds>(downed).map_or(0, |w| w.len());

    let volley = fire_volley(
        &mut world,
        shooter,
        mode,
        &occupancy,
        front_cell(),
        0xD09E_D517,
    );

    assert!(
        report_struck(&volley, downed),
        "a round at the floor must strike the downed body lying there — got {:?}",
        volley.reports,
    );
    let wounds_after = world.get::<InflictedWounds>(downed).map_or(0, |w| w.len());
    assert!(
        wounds_after > wounds_before,
        "a downed ganger keeps taking wounds — InflictedWounds went {wounds_before} → \
         {wounds_after}",
    );
}

#[test]
fn a_round_stopping_on_a_corpse_leaves_it_untouched() {
    let mut world = World::new();
    let mode = burst_mode(1);
    let shooter = spawn_shooter(&mut world, mode);
    let corpse = line_ganger(&mut world, front_cell(), 0, LifeState::Dead);
    let live = line_ganger(&mut world, behind_cell(), 6, LifeState::Alive);

    let mut occupancy = OccupancyGrid::new();
    place_body(&mut occupancy, front_cell(), corpse, HeightBand::Low);
    place_occupant(&mut occupancy, behind_cell(), live, HeightBand::Low);

    let hp_before = world.get::<Hp>(corpse).copied();

    let volley = fire_volley(
        &mut world,
        shooter,
        mode,
        &occupancy,
        behind_cell(),
        0xC0FF_EE17,
    );

    assert!(
        report_struck(&volley, corpse),
        "the floor-level round must stop on the corpse — got {:?}",
        volley.reports,
    );
    let corpse_wounds = world.get::<InflictedWounds>(corpse).map_or(0, |w| w.len());
    assert_eq!(
        corpse_wounds, 0,
        "a corpse a round stops on records NO wound — found {corpse_wounds}",
    );
    assert_eq!(
        world.get::<Hp>(corpse).copied(),
        hp_before,
        "a corpse a round stops on loses no Hp — found {:?}",
        world.get::<Hp>(corpse).copied(),
    );
}
