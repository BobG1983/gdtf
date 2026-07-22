//! GTW-227 (GTW-48 S8 / 222b): headless integration tests for the core player ACTS —
//! fire (left-click), posture (stance / aim / facing keys), and fire-mode selection —
//! over the REAL 222a act-intent queue (`GdtfBattleInputPlugin`'s keyboard / fire-click
//! systems -> the ONE `dispatch_act_intents` drain -> the emitted `*Requested`).
//!
//! - AC1 drives the REAL selection path (synth left-click on an armed occupant) and
//!   asserts `SelectedFireMode` defaults to that weapon's `FireMode::single()`.
//! - (GTW-254) The blind fire-mode cycle was REMOVED — the `gdtf_app` popup picker
//!   replaced it, so the old AC2 key-walk test is gone (the picker's behavior is covered
//!   by the `gdtf_app` `action_bar.rs` integration tests). `sync_fire_mode_on_select`
//!   (AC1) is still the picker's default-on-select dependency.
//! - AC3 synthesizes a left-click on an in-bounds target for an alive / loaded /
//!   affordable shooter and asserts EXACTLY one `FireRequested` with the expected
//!   shooter / mode / target fields is emitted (a probe `MessageReader`).
//! - AC4 varies ONE failing `can_fire` input at a time (Downed; empty magazine; TU one
//!   below the charge; an out-of-bounds target) and asserts ZERO `FireRequested`.
//! - AC5 synthesizes each posture key and asserts one `SetStanceRequested` /
//!   `SetAimingRequested` / `SetFacingRequested` for `*SelectedShooter` with the
//!   next-of-cycle value — byte-for-byte EQUAL to the message the SAME `ActIntent`
//!   pushed directly (the 222c-button surrogate) produces.
//! - AC6 clears the selection and drives all act keys + a left-click, asserting ZERO
//!   messages of every `*Requested` type and no panic.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

mod downed;
mod emplacement;
mod fire;
mod gating;
mod harness;
mod move_two_click;
mod open_door;
mod posture;
mod view_toggle;
