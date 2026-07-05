//! Right-click turn-to-face + enemy-selection inertness + drain-exactly-once
//! (AC5/AC6/AC7).

use bevy::prelude::*;
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_battle_sim::{
    Cell, CellLevel, Direction, Position,
    acts::{MoveRequested, SetFacingRequested},
};
use gdtf_test_utils::press_mouse;

use super::harness::*;

// ---------------------------------------------------------------------------------
// AC5 — right-click + selection -> SetFacingRequested to from_cells(actor, hovered).
// ---------------------------------------------------------------------------------

/// AC5 — with a player-faction selection on `Position` cell `(5,5)` and the `InspectTarget`
/// hovered cell on
/// `(8,5)`, one Right press emits exactly one `SetFacingRequested { facing = East }`
/// (`Direction::from_cells((5,5),(8,5)) == East`). Hovering the actor's OWN cell emits no
/// intent.
#[test]
fn right_click_turns_to_face_the_hovered_cell() {
    // (5,5) -> (8,5) is due East.
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
    // Hovering the actor's OWN cell -> from_cells None -> no intent.
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

// ---------------------------------------------------------------------------------
// AC6 — gating covers Left AND Right: a forced ENEMY selection emits NOTHING.
// ---------------------------------------------------------------------------------

/// AC6 — a FORCED enemy-faction `SelectedShooter` (a non-player entity injected as the
/// selection) emits NOTHING on a Left press (no Move/Fire) AND nothing on a Right press
/// (no `SetFacingRequested`), and is never treated as a player actor.
#[test]
fn forced_enemy_selection_emits_nothing_on_left_or_right() {
    // Forced enemy selection -> Left press over an empty cell emits nothing.
    {
        let mut app = control_app();
        // An enemy ganger with a Position (so right-click's Position lookup succeeds and
        // the ONLY block is the faction gate).
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

        // Right press over a distinct cell — no turn either.
        set_hovered(&mut app, Some(CellLevel::new(Cell::new(25, 20), LEVEL)));
        press_mouse(&mut app, MouseButton::Right);
        app.update();
        assert!(
            facings(&app).is_empty(),
            "a forced enemy selection must emit no SetFacingRequested on a Right press",
        );
    }
}

// ---------------------------------------------------------------------------------
// AC7 — the drain emits each NEW intent exactly once and empties the queue.
// ---------------------------------------------------------------------------------

/// AC7 — queueing an `ActIntent::Move` and an `ActIntent::Turn` (via the public
/// `PendingActIntent::push`) and running ONE update emits exactly one `MoveRequested` and
/// one `SetFacingRequested`, and the queue is `is_empty()` afterward (take-and-clear).
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
