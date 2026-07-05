//! GTW-371 (C2 / C4): headless integration tests for the input-crate `populate_fire_target`
//! system — it POPULATES the presenter-owned `FireTargetHighlight` when the SELECTED player
//! shooter hovers a squad-VISIBLE ENEMY it could fire on (mirroring `decide_left_click`'s FIRE
//! rung + the GTW-346 fog gate), exposing the `mode_tu_cost` the shot would charge, and is WIRED
//! in the input plugin.
//!
//! POSITIVE — every assertion NAMES the cell + computes the expected cost from `mode_tu_cost`
//! (no magnitude pin):
//!
//! - C2 / C4b (the producer): hovering a fireable enemy fills `FireTargetHighlight` with the
//!   hovered cell + a cost EXACTLY equal to `mode_tu_cost(SelectedFireMode, TuMax, Aiming,
//!   CombatTuning)` (computed independently in-test). Selection / fire-mode / hover are never
//!   written into the sim.
//! - C4c (clears): NOT hovering a fireable enemy (empty cell / own ganger / non-visible enemy /
//!   no selection) clears the highlight (empty).
//!
//! The click systems run `.before(pick_hovered_cell)`, so an INJECTED `InspectTarget` is read
//! that update before the (headless, camera-less) picker clobbers it to `None`. Every
//! `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom (`bevy-traps.md`
//! #7 carve-out (a)).

mod clears;
mod harness;
mod populates;
