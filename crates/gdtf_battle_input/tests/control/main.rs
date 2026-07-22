//! GTW-238: headless integration tests for the `PlayerFaction`-gated control surface —
//! the ONE disambiguated left-click decision (`left_click_act`: FIRE -> SELECT -> MOVE
//! -> CLEAR) and the right-click turn-to-face surface (`right_click_turn_to_face`), over
//! the REAL `GdtfBattleInputPlugin` drain (its click systems -> the ONE
//! `dispatch_act_intents` drain -> the emitted `*Requested`).
//!
//! Tests are headless `GdtfBattleInputPlugin` apps: synth `ButtonInput<MouseButton>` +
//! `InspectTarget` + `OccupancyGrid` + `PlayerFaction` + spawned `Faction`/`Position`/firing
//! components, `app.update()`, assert the emitted `*Requested` / `SelectedShooter`. The
//! click systems run `.before(pick_hovered_cell)`, so an INJECTED `InspectTarget` is read
//! that update before the (headless, camera-less) picker clobbers it to `None`.
//!
//! - AC1 — left-click SELECTS only a player ganger (enemy/empty does not select-as-own).
//! - AC2 — left-click empty + player selection -> exactly one `MoveRequested`, no fire.
//! - AC3 — left-click ENEMY + fire mode -> exactly one `FireRequested`, no move, selection
//!   unchanged.
//! - AC4 — left-click EMPTY + fire mode + selection -> falls through to MOVE (no fire).
//! - AC5 — right-click + selection -> `SetFacingRequested` to `from_cells(actor, hovered)`;
//!   the actor's own cell is a no-op.
//! - AC6 — a FORCED enemy selection emits NOTHING on Left or Right.
//! - AC7 — the drain emits each NEW intent (`Move`/`Turn`) exactly once and empties the
//!   queue.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

mod fire;
mod harness;
mod select_and_move;
mod turn_and_drain;
