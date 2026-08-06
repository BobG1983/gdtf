use gdtf_battle_sim::{
    ganger::Direction,
    metric::Cell,
    prelude::{Stance, StanceKind},
};

use crate::dev::net_qa::wire::{
    act_payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet},
    cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
    test::assert_ron_round_trip,
    token::GangerToken,
};

/// Every wire facing, beside the sim direction it names and the cell that direction steps to.
const FACINGS: [(FacingNet, Direction, i32, i32); 8] = [
    (FacingNet::North, Direction::North, 0, -1),
    (FacingNet::NorthEast, Direction::NorthEast, 1, -1),
    (FacingNet::East, Direction::East, 1, 0),
    (FacingNet::SouthEast, Direction::SouthEast, 1, 1),
    (FacingNet::South, Direction::South, 0, 1),
    (FacingNet::SouthWest, Direction::SouthWest, -1, 1),
    (FacingNet::West, Direction::West, -1, 0),
    (FacingNet::NorthWest, Direction::NorthWest, -1, -1),
];

/// Every wire stance beside the sim posture it names.
const STANCES: [(StanceNet, StanceKind); 3] = [
    (StanceNet::Standing, StanceKind::Standing),
    (StanceNet::Crouching, StanceKind::Crouching),
    (StanceNet::Prone, StanceKind::Prone),
];

fn a_cell_level() -> CellLevelNet {
    CellLevelNet::new(
        CellNet::new(CellXNet::new(3), CellYNet::new(4)),
        LevelNet::new(2),
    )
}

#[test]
fn stance_net_round_trips_every_variant() {
    for (stance, _) in STANCES {
        match stance {
            StanceNet::Standing | StanceNet::Crouching | StanceNet::Prone => {}
        }
        assert_ron_round_trip(&stance);
    }
}

#[test]
fn every_stance_names_the_sim_posture_of_the_same_name() {
    for (stance, kind) in STANCES {
        assert_eq!(
            stance.to_sim(),
            kind,
            "`act.set_stance` hands the sim whatever `to_sim` returns, so a crossed arm would \
             put the ganger in a posture nobody asked for: {stance:?}",
        );
        assert_eq!(
            StanceNet::from_sim(Stance::new(kind)),
            stance,
            "a card reports the posture the sim holds, so the mirror must be lossless both ways: \
             {kind:?}",
        );
    }
}

#[test]
fn facing_net_round_trips_every_variant() {
    for (facing, ..) in FACINGS {
        match facing {
            FacingNet::North
            | FacingNet::NorthEast
            | FacingNet::East
            | FacingNet::SouthEast
            | FacingNet::South
            | FacingNet::SouthWest
            | FacingNet::West
            | FacingNet::NorthWest => {}
        }
        assert_ron_round_trip(&facing);
    }
}

#[test]
fn every_facing_names_the_sim_direction_that_steps_that_way() {
    for (facing, direction, east, south) in FACINGS {
        assert_eq!(
            facing.to_sim(),
            direction,
            "`act.set_facing` turns the ganger to whatever `to_sim` returns, so a crossed arm \
             would point it the wrong way: {facing:?}",
        );
        assert_eq!(
            FacingNet::from_sim(direction),
            facing,
            "a read reports the direction the sim holds, so the mirror must be lossless both \
             ways: {direction:?}",
        );
        assert_eq!(
            direction.cell_step(),
            Cell::new(east, south),
            "the wire name is the compass point the sim steps toward, not a label the table is \
             free to move: {facing:?}",
        );
    }
}

#[test]
fn melee_target_round_trips_every_variant() {
    for target in [
        MeleeTargetNet::Ganger(GangerToken::new(1)),
        MeleeTargetNet::Structure(a_cell_level()),
    ] {
        match target {
            MeleeTargetNet::Ganger(_) | MeleeTargetNet::Structure(_) => {}
        }
        assert_ron_round_trip(&target);
    }
}

#[test]
fn an_aim_flag_round_trips_and_carries_the_value_it_was_built_from() {
    for aim in [true, false] {
        let wrapped = AimNet::new(aim);
        assert_ron_round_trip(&wrapped);
        assert_eq!(
            *wrapped, aim,
            "`act.set_aiming` reads the flag straight back out of the wrapper on its way to the \
             sim, so the wrapper must not invert it",
        );
    }
}
