//! The Enter button follows the seat's entry sides: no side named, no offer anywhere.

use bevy::prelude::*;
use gdtf_battle_sim::{emplacement::EmplacementState, prelude::Position};
use gdtf_game::test_support::EnterEmplacementButton;

use super::{actors::*, harness::*};

/// The eight neighbours of a cell, as the offsets that name them.
const AROUND: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// The four cardinal offsets, the cells an open mount's entry sides name.
const CARDINALS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// Stand the actor on a cell and let the panel's offer systems answer for it.
fn stand(app: &mut App, actor: Entity, x: i32, y: i32) {
    if let Some(mut position) = app.world_mut().get_mut::<Position>(actor) {
        *position = at(x, y);
    }
    app.update();
}

#[test]
fn a_seat_naming_no_entry_side_offers_enter_from_none_of_its_eight_neighbours() {
    let mut app = battle_running_app();
    let actor = spawn_emplacement_actor(&mut app, 20, 20, 0);
    let (seat_x, seat_y) = (20, 20);
    spawn_emplacement(
        &mut app,
        seat_x,
        seat_y,
        EmplacementState::Vacant,
        None,
        None,
        None,
    );

    for (east, north) in AROUND {
        stand(&mut app, actor, seat_x + east, seat_y + north);
        assert_eq!(
            visibility::<EnterEmplacementButton>(&mut app),
            Some(Visibility::Hidden),
            "a seat carrying no entry sides at all offers no Enter, including from its \
             ({east}, {north}) neighbour",
        );
    }
}

#[test]
fn a_seat_naming_the_four_cardinals_offers_enter_from_those_four_cells_only() {
    let mut app = battle_running_app();
    let actor = spawn_emplacement_actor(&mut app, 20, 20, 0);
    let (seat_x, seat_y) = (20, 20);
    spawn_emplacement(
        &mut app,
        seat_x,
        seat_y,
        EmplacementState::Vacant,
        None,
        Some(cardinal_sides()),
        Some(cardinal_facing()),
    );

    for (east, north) in AROUND {
        stand(&mut app, actor, seat_x + east, seat_y + north);
        let cardinal = CARDINALS.contains(&(east, north));
        let wanted = if cardinal {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        assert_eq!(
            visibility::<EnterEmplacementButton>(&mut app),
            Some(wanted),
            "a seat naming the four cardinals offers Enter from its cardinal neighbours and \
             from nowhere else; the ({east}, {north}) neighbour is {}",
            if cardinal { "cardinal" } else { "diagonal" },
        );
    }
}
