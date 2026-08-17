//! Throw stays offered without a live hover, and a press keeps the last cell.

use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level},
    weapon::TrajectoryStyle,
};
use gdtf_test_utils::press_ui_button;

use super::{
    harness::battle_running_app,
    throw::{
        add_throw_probe, clear_hover, hover_cell, spawn_throw_actor, the_throw_button,
        throw_visible, throws,
    },
};

#[test]
fn an_arc_weapon_offers_throw_with_nothing_hovered() {
    let mut app = battle_running_app();
    spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Arc);
    clear_hover(&mut app);
    app.update();

    assert!(
        throw_visible(&mut app),
        "an Arc weapon with a loaded magazine offers Throw with no cell hovered",
    );
}

#[test]
fn throw_stays_offered_after_the_hover_clears() {
    let mut app = battle_running_app();
    spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Arc);
    hover_cell(&mut app, 15, 15);
    app.update();
    assert!(
        throw_visible(&mut app),
        "sanity: a hovered cell offers Throw",
    );

    clear_hover(&mut app);
    app.update();
    assert!(
        throw_visible(&mut app),
        "clearing the hover must leave Throw up while the ganger still has something to throw",
    );
}

#[test]
fn pressing_throw_after_hover_clears_aims_at_the_last_cell() {
    let mut app = battle_running_app();
    add_throw_probe(&mut app);
    let thrower = spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Arc);
    let target = CellLevel::new(Cell::new(15, 15), Level::new(0));
    hover_cell(&mut app, 15, 15);
    app.update();
    let throw_btn = the_throw_button(&mut app);

    clear_hover(&mut app);
    press_ui_button(&mut app, throw_btn);
    app.update();

    let emitted = throws(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Throw after the hover left the map must still emit one ThrowGrenadeRequested",
    );
    assert_eq!(
        emitted[0].thrower, thrower,
        "the thrower is the SelectedShooter",
    );
    assert_eq!(
        emitted[0].target, target,
        "the target is the last hovered cell, not whatever the pointer sits on now",
    );
}

#[test]
fn pressing_throw_with_no_cell_ever_hovered_emits_nothing() {
    let mut app = battle_running_app();
    add_throw_probe(&mut app);
    spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Arc);
    clear_hover(&mut app);
    app.update();
    assert!(
        throw_visible(&mut app),
        "sanity: Throw is offered before any cell is hovered",
    );
    let throw_btn = the_throw_button(&mut app);

    press_ui_button(&mut app, throw_btn);
    app.update();

    assert_eq!(
        throws(&app).len(),
        0,
        "a press with no cell ever hovered must not invent a landing cell",
    );
}
