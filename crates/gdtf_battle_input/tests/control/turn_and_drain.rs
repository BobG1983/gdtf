use bevy::prelude::*;
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_battle_sim::{
    acts::{MoveRequested, SetFacingRequested},
    prelude::{Cell, CellLevel, Direction, Position},
};
use gdtf_test_utils::press_mouse;

use super::harness::*;


#[test]
fn right_click_turns_to_face_the_hovered_cell() {
    {
        let mut app = control_app();
        let actor_cell = CellLevel::new(Cell::new(5, 5), LEVEL);
        let ganger = spawn_player_shooter(&mut app, actor_cell);
        set_selection(&mut app, ganger);
        set_hovered(&mut app, Some(CellLevel::new(Cell::new(8, 5), LEVEL)));

        press_mouse(&mut app, MouseButton::Right);
        app.update();

        let emitted = facings(&app);
        assert_eq!(
            emitted.len(),
            1,
            "exactly one SetFacingRequested on a Right press with a player selection",
        );
        assert_eq!(emitted[0].actor, ganger, "the turn actor = the selection");
        assert_eq!(
            emitted[0].facing,
            Direction::East,
            "from_cells((5,5),(8,5)) must be East",
        );
    }
    {
        let mut app = control_app();
        let actor_cell = CellLevel::new(Cell::new(5, 5), LEVEL);
        let ganger = spawn_player_shooter(&mut app, actor_cell);
        set_selection(&mut app, ganger);
        set_hovered(&mut app, Some(actor_cell));

        press_mouse(&mut app, MouseButton::Right);
        app.update();

        assert!(
            facings(&app).is_empty(),
            "hovering the actor's own cell must emit no SetFacingRequested",
        );
    }
}


#[test]
fn forced_enemy_selection_emits_nothing_on_left_or_right() {
    {
        let mut app = control_app();
        let enemy_cell = CellLevel::new(Cell::new(20, 20), LEVEL);
        let enemy = app
            .world_mut()
            .spawn((ENEMY_FACTION, Position::new(enemy_cell)))
            .id();
        set_selection(&mut app, enemy);
        set_hovered(&mut app, Some(CellLevel::new(Cell::new(21, 20), LEVEL)));

        press_mouse(&mut app, MouseButton::Left);
        app.update();
        assert!(
            moves(&app).is_empty() && fires(&app).is_empty(),
            "a forced enemy selection must emit no Move/Fire on a Left press",
        );

        set_hovered(&mut app, Some(CellLevel::new(Cell::new(25, 20), LEVEL)));
        press_mouse(&mut app, MouseButton::Right);
        app.update();
        assert!(
            facings(&app).is_empty(),
            "a forced enemy selection must emit no SetFacingRequested on a Right press",
        );
    }
}


#[test]
fn drain_emits_each_new_intent_exactly_once() {
    let mut app = control_app();
    let actor = spawn_player_shooter(&mut app, CellLevel::new(Cell::new(1, 1), LEVEL));
    let dest = CellLevel::new(Cell::new(2, 1), LEVEL);

    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::Move(MoveRequested::new(actor, dest)));
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::Turn(SetFacingRequested::new(
            actor,
            Direction::East,
        )));

    app.update();

    assert_eq!(
        moves(&app).len(),
        1,
        "the drain must emit exactly one MoveRequested",
    );
    assert_eq!(
        facings(&app).len(),
        1,
        "the drain must emit exactly one SetFacingRequested",
    );
    assert!(
        app.world()
            .get_resource::<PendingActIntent>()
            .is_some_and(PendingActIntent::is_empty),
        "the intent queue must be empty after the drain (take-and-clear)",
    );
}
