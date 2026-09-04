//! The door half of the lit area: which doors reach it, and in which state.

use gdtf_battle_sim::openable::OpenState;
use gdtf_game::qa_wire::{token::DoorToken, visible::DoorOpenNet};

use super::body::visible_with_a_door;
use crate::{battle_setup::Standing, socket_support::TestResult};

#[test]
fn an_open_door_in_the_lit_area_is_listed_open() -> TestResult {
    let (visible, door) = visible_with_a_door(Standing::Lit, OpenState::Open)?;
    let token = DoorToken::new(door.entity.to_bits());

    let Some(entry) = visible.doors.iter().find(|entry| entry.token == token) else {
        unreachable!("a door on a lit cell is in the lit area: {door:?} missing from {visible:?}");
    };
    assert_eq!(
        entry.at, door.at,
        "the entry names the cell the door stands on: {entry:?}",
    );
    assert_eq!(
        entry.open,
        DoorOpenNet::new(true),
        "the entry reports the door's own open state: {entry:?}",
    );
    Ok(())
}

#[test]
fn a_closed_door_in_the_lit_area_is_listed_closed() -> TestResult {
    let (visible, door) = visible_with_a_door(Standing::Lit, OpenState::Closed)?;
    let token = DoorToken::new(door.entity.to_bits());

    let Some(entry) = visible.doors.iter().find(|entry| entry.token == token) else {
        unreachable!(
            "a closed door on a lit cell is in the lit area too: {door:?} missing from \
             {visible:?}"
        );
    };
    assert_eq!(
        entry.open,
        DoorOpenNet::new(false),
        "a map seeds its doors closed, so the read must report that state rather than a \
         constant: {entry:?}",
    );
    Ok(())
}

#[test]
fn a_door_the_fog_hides_is_left_out_of_the_lit_area() -> TestResult {
    let (visible, door) = visible_with_a_door(Standing::Hidden, OpenState::Open)?;
    let token = DoorToken::new(door.entity.to_bits());

    assert!(
        !visible.doors.iter().any(|entry| entry.token == token),
        "the door list is the lit area only, so a door the fog hides never reaches the wire: \
         {door:?} appears in {visible:?}",
    );
    Ok(())
}

#[test]
fn the_door_list_comes_back_ordered() -> TestResult {
    let (visible, door) = visible_with_a_door(Standing::Lit, OpenState::Open)?;

    let doors: Vec<u64> = visible.doors.iter().map(|entry| *entry.token).collect();
    assert!(
        !doors.is_empty(),
        "the fixture lights a door, so this case has a list to check: {door:?}",
    );
    assert!(
        doors.is_sorted(),
        "the door list is sorted before it goes out: {doors:?}",
    );
    Ok(())
}
