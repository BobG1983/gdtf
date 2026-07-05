//! The left-click SELECT rung + the two-click MOVE targeting / retarget
//! (AC1/AC2).

use bevy::prelude::*;
use gdtf_battle_sim::{Cell, CellLevel};
use gdtf_test_utils::{clear_mouse, press_mouse};

use super::harness::*;

// ---------------------------------------------------------------------------------
// AC1 — left-click SELECTS only a player ganger.
// ---------------------------------------------------------------------------------

/// AC1 — a Left press over a PLAYER-faction occupant selects it; a Left press over a
/// NON-player occupant (or an empty cell) does NOT become a player-own selection.
#[test]
fn left_click_selects_only_a_player_ganger() {
    // Player occupant -> selected.
    {
        let mut app = control_app();
        let cell = CellLevel::new(Cell::new(5, 5), LEVEL);
        let ganger = spawn_player_shooter(&mut app, cell);
        set_hovered(&mut app, Some(cell));
        press_mouse(&mut app, MouseButton::Left);
        app.update();
        assert_eq!(
            selected(&app),
            Some(ganger),
            "a Left press over a player-faction occupant must select it",
        );
    }
    // Enemy occupant -> NOT selected-as-own (no fire mode forces fire; no prior selection
    // -> CLEAR).
    {
        let mut app = control_app();
        let cell = CellLevel::new(Cell::new(7, 7), LEVEL);
        let enemy = place_enemy(&mut app, cell);
        set_hovered(&mut app, Some(cell));
        press_mouse(&mut app, MouseButton::Left);
        app.update();
        assert_ne!(
            selected(&app),
            Some(enemy),
            "an enemy occupant must never become a player-own selection",
        );
    }
    // Empty cell -> not a selection.
    {
        let mut app = control_app();
        set_hovered(&mut app, Some(CellLevel::new(Cell::new(9, 9), LEVEL)));
        press_mouse(&mut app, MouseButton::Left);
        app.update();
        assert_eq!(
            selected(&app),
            None,
            "a Left press over an empty cell must not select",
        );
    }
}

// ---------------------------------------------------------------------------------
// AC2 (GTW-356) — left-click empty + player selection is TWO-CLICK: click-1 SETS the
// move target (no dispatch); click-2 on the SAME cell COMMITS the move.
// ---------------------------------------------------------------------------------

/// AC2 — with a player-faction `SelectedShooter`, the FIRST Left press on an empty,
/// in-bounds, unblocked cell SETS `PathPreviewTarget` to that cell and emits NO
/// `MoveRequested`; the SECOND Left press on the SAME cell emits exactly one `MoveRequested
/// { actor = selection, dest = hovered }`, clears the target, and emits zero `FireRequested`
/// (GTW-356 two-click flow — the single-click immediate-move was REPLACED, not weakened).
#[test]
fn two_click_empty_with_selection_targets_then_moves() {
    let mut app = control_app();
    let shooter_cell = CellLevel::new(Cell::new(3, 3), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, ganger);

    let dest = CellLevel::new(Cell::new(4, 3), LEVEL);
    set_hovered(&mut app, Some(dest));

    // Click-1: SET the target, dispatch NOTHING.
    press_mouse(&mut app, MouseButton::Left);
    app.update();
    assert!(
        moves(&app).is_empty(),
        "click-1 on a valid target must emit NO MoveRequested (it only sets the target)",
    );
    assert_eq!(
        move_target(&app),
        Some(dest),
        "click-1 must SET PathPreviewTarget to the clicked cell",
    );

    // Click-2 on the SAME cell: COMMIT.
    clear_mouse(&mut app);
    set_hovered(&mut app, Some(dest));
    press_mouse(&mut app, MouseButton::Left);
    app.update();

    let emitted = moves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "click-2 on the SAME cell must emit exactly one MoveRequested (commit)",
    );
    assert_eq!(emitted[0].actor, ganger, "move actor = the selection");
    assert_eq!(emitted[0].dest, dest, "move dest = the targeted cell");
    assert_eq!(
        move_target(&app),
        None,
        "committing the move must CLEAR PathPreviewTarget",
    );
    assert!(
        fires(&app).is_empty(),
        "a MOVE commit must emit no FireRequested",
    );
}

/// AC2 (GTW-356 re-target) — with a target already pending, a click on a DIFFERENT valid
/// cell RE-TARGETS the preview (does NOT commit): no `MoveRequested`, and the target follows
/// the new cell. (The same-cell-commit / different-cell-retarget interpretation, FLAGGED.)
#[test]
fn click_on_a_different_cell_retargets_without_committing() {
    let mut app = control_app();
    let shooter_cell = CellLevel::new(Cell::new(3, 3), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, ganger);

    let first = CellLevel::new(Cell::new(4, 3), LEVEL);
    set_hovered(&mut app, Some(first));
    press_mouse(&mut app, MouseButton::Left);
    app.update();
    assert_eq!(
        move_target(&app),
        Some(first),
        "click-1 sets the first target"
    );

    // A click on a DIFFERENT valid cell re-targets, never commits.
    clear_mouse(&mut app);
    let second = CellLevel::new(Cell::new(5, 3), LEVEL);
    set_hovered(&mut app, Some(second));
    press_mouse(&mut app, MouseButton::Left);
    app.update();

    assert!(
        moves(&app).is_empty(),
        "a click on a DIFFERENT cell must RE-TARGET, never commit (no MoveRequested)",
    );
    assert_eq!(
        move_target(&app),
        Some(second),
        "the target must follow the newly-clicked cell",
    );
}
