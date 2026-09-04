//! What the sightline read answers with and without a selected shooter.

use bevy::{ecs::system::RunSystemOnce as _, prelude::*};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    ganger::{Aiming, Direction, Facing, LifeState, Position, Tu, TuMax},
    prelude::{Cell, CellLevel, Level},
};

use crate::dev::mcp::{
    commands::read::battle_sightline::SightlineReads, wire::sight::SightlineNet,
};

fn a_cell() -> CellLevel {
    CellLevel::new(Cell::new(4, 5), Level::new(0))
}

/// Ask the handler's own decision, through the real system param.
fn answer(world: &mut World) -> SightlineNet {
    let asked = world.run_system_once(|reads: SightlineReads| reads.sightline(a_cell()));
    match asked {
        Ok(sightline) => sightline,
        Err(fault) => unreachable!("the sightline read is a plain read-only system: {fault:?}"),
    }
}

#[test]
fn a_battle_with_nothing_selected_answers_no_shooter() {
    let mut world = World::new();
    world.insert_resource(SelectedShooter::cleared());

    assert_eq!(
        answer(&mut world),
        SightlineNet::NoShooter,
        "with nothing selected there is no shooter to price the answer for, so the reply says \
         so rather than answering false",
    );
}

#[test]
fn a_host_with_no_selection_resource_at_all_answers_no_shooter() {
    let mut world = World::new();

    assert_eq!(
        answer(&mut world),
        SightlineNet::NoShooter,
        "a host that never built the selection resource is the same case as an empty \
         selection",
    );
}

#[test]
fn a_battle_with_a_shooter_selected_answers_both_halves() {
    let mut world = World::new();
    let shooter = world
        .spawn((
            Position::new(a_cell()),
            Facing::new(Direction::North),
            Tu::new(30),
            TuMax::new(60),
            Aiming::default(),
            LifeState::Alive,
        ))
        .id();
    world.insert_resource(SelectedShooter::new(shooter));

    let sightline = answer(&mut world);

    assert!(
        matches!(sightline, SightlineNet::Answered { .. }),
        "with a shooter selected the reply carries both halves, never the refusal: \
         {sightline:?}",
    );
}
