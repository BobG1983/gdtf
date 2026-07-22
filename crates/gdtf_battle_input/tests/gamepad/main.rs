//! GTW-259: tests for the gamepad software cursor — the SHARED decision (AC2), the picker
//! arbitration (AC3), the mouse-reclaims-pointer logic (AC4), and the gamepad acts reusing
//! the intent queue (AC6, by construction + the shared decision).
//!
//! HONESTY (the contract / `verification.md`): real `Gamepad` stick / button STATE is
//! device-event-driven in Bevy 0.18 and cannot be cleanly driven headlessly (the same
//! limitation GTW-250 documents). So the raw stick / button READS (`move_gamepad_cursor`,
//! `gamepad_click_act`, `gamepad_turn`) are covered by the pure `move_cursor` helper (its
//! unit tests live in `gamepad.rs`) + the SHARED decision exercised here via the mouse /
//! `SystemState` path + in-engine QA. These headless tests prove the SETTABLE-resource /
//! message logic: the picker honors `ActivePointer`, the mouse reclaims the pointer, and the
//! shared `decide_left_click` / `decide_turn` resolve the contract precedence.
//!
//! Every `app.world_mut()` / `SystemState` use is over an [`App`] — the accepted headless
//! idiom (`bevy-traps.md` #7 carve-out (a)): the AC2 shared-decision tests build the same
//! `picking_app()`-style [`App`] (`MinimalPlugins` + the real [`GdtfBattleInputPlugin`]) the
//! AC3/AC4 tests use, spawn fixtures with `app.world_mut().spawn(...)`, and read the shared
//! decision via a [`SystemState`] constructed against `app.world_mut()`. No helper here takes
//! `&mut World` / `&World` in its signature (the landed sibling `selection.rs` house style).

mod decision;
mod fog_gate;
mod harness;
mod pointer;
