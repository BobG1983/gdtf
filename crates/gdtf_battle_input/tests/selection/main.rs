//! GTW-225 (GTW-48 S8): headless integration tests for ganger selection, the
//! selection highlight, level cycling, and the shared act-intent queue — updated for the
//! GTW-238 PlayerFaction-gated unified left-click decision (`left_click_act` replaces
//! `select_on_click`).
//!
//! - AC1 proves `GdtfBattleInputPlugin` `init_resource`s `SelectedShooter` (present +
//!   `None` after one update) and the `PendingActIntent` queue.
//! - drives the REAL `left_click_act` system: a synthesized `ButtonInput<MouseButton>`
//!   press + an `InspectTarget` + an `OccupancyGrid` PLAYER-faction occupant selects that
//!   occupant; an empty cell clears to `None`. GTW-238 gates SELECT to the player
//!   faction (an enemy occupant is NOT selected — covered in `control.rs`).
//! - drives `update_selection_highlight`: the one `SelectionHighlight` sprite snaps to
//!   `cell_to_world(selected cell)` visible, and hides on clear.
//! - AC6/AC9 drive level cycling THROUGH the queue: pushing a level-up intent +
//!   update mutates `ActiveLevel`, saturating at `MAX_LEVELS - 1` and flooring at 0.
//! - AC6 (keyboard real path) drives the REAL `level_keys` / `select_clear_key`
//!   systems: with a `Keybinds` resource inserted, a synthesized
//!   `ButtonInput<KeyCode>` just-pressed of the BOUND level-up / clear key flows
//!   key-press -> intent push -> `dispatch_act_intents` drain -> `ActiveLevel` /
//!   `SelectedShooter`, end-to-end (reverting either keyboard system fails these).
//! - AC11 proves the input layer is inert WITHOUT `BattleInProgress`.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

mod auto_select;
mod click_select;
mod gating;
mod harness;
mod highlight;
mod intent_seam;
mod real_flow;
