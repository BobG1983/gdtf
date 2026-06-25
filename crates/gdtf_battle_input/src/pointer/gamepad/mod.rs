//! Gamepad software cursor (GTW-259): a stick-driven screen cursor that drives the SAME
//! select / move / fire / turn path the mouse does, so a gamepad can play the battle.
//!
//! Fixes play-test bug #7 (gamepad half): battle control was mouse-only. This module adds a
//! SOFTWARE cursor steered by the gamepad LEFT stick that:
//!
//! 1. moves a [`GamepadCursor`] screen position each update by the stick × [`CursorSpeed`] × `dt`
//!    (the y-FLIPped [`move_cursor`] pure helper), claiming the [`ActivePointer`] for the gamepad
//!    whenever the stick passes the [`CursorStickDeadzone`];
//! 2. lets the generalized picker ([`pick_hovered_cell`](crate::pick_hovered_cell)) project THAT
//!    cursor when the gamepad is the active pointer — so the landed GTW-251 highlight follows the
//!    gamepad cursor for free (NO separate reticle);
//! 3. resolves South (left-click equivalent) through the SHARED
//!    [`decide_left_click`](crate::selection::decide_left_click) /
//!    [`apply_left_click`](crate::selection::apply_left_click) and East (right-click equivalent)
//!    through the SHARED [`decide_turn`](crate::selection::decide_turn) — ONE precedence
//!    implementation, two devices; and
//! 4. emits the cursor's screen position as a presenter-defined [`GamepadCursorMoved`](gdtf_battle_presenter::GamepadCursorMoved)
//!    message so the presenter can EDGE-PAN (input→presenter, no cycle — the `HighlightRequest`
//!    precedent).
//!
//! # Mouse + gamepad coexist (last-moved-input wins)
//!
//! [`ActivePointer`] arbitrates: the stick passing the deadzone sets [`ActivePointer::Gamepad`]; a
//! [`CursorMoved`](bevy::window::CursorMoved) message (the OS mouse moved) sets
//! [`ActivePointer::Mouse`] ([`mouse_reclaims_pointer`]). The picker reads it to choose which
//! cursor to project, so whichever device moved last drives the highlighted cell.
//!
//! # South / East are hardcoded (the `focus_nav` GTW-119 precedent)
//!
//! South = left-click / act, East = right-click / turn — read directly off the [`Gamepad`](bevy::input::gamepad::Gamepad)
//! component, the same way `gdtf_ui::focus_nav` reads `GamepadButton::South` for activation.
//! Gamepad remapping is a future ticket.
//!
//! # Honesty: raw pad state is not headlessly drivable
//!
//! Real [`Gamepad`](bevy::input::gamepad::Gamepad) stick / button STATE is device-event-driven in
//! Bevy 0.18 and cannot be cleanly driven from a headless test (the same limitation GTW-250
//! documents). So the raw stick / button READS ([`move_gamepad_cursor`], [`gamepad_click_act`],
//! [`gamepad_turn`]) are covered by the pure [`move_cursor`] helper + the shared decision (exercised
//! via the mouse path) + in-engine QA; the headless tests prove the SETTABLE-resource / message
//! logic.

mod cursor;
mod systems;

#[cfg(test)]
mod test;

pub use cursor::{
    ActivePointer, CURSOR_SPEED, CURSOR_STICK_DEADZONE, CursorSpeed, CursorStickDeadzone,
    GamepadCursor, move_cursor,
};
pub use systems::{
    emit_gamepad_cursor_move, gamepad_click_act, gamepad_turn, mouse_reclaims_pointer,
    move_gamepad_cursor,
};
