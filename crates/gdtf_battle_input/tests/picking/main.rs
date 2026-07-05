//! GTW-221 (GTW-48 S7) + GTW-251: headless integration tests for the cursor->cell
//! picking and the message-driven hover-highlight EMIT.
//!
//! - AC1 (build-ran) proves `GdtfBattleInputPlugin`'s `build` runs inside the REAL
//!   scene stack: the `GdtfTestAppBuilder` (`MinimalPlugins` + the real `ScenesPlugin`
//!   state machine) descends to `GameState::BattleScape` and the plugin's
//!   `GdtfBattleInputActive` marker is present — the exact `presenter_foundation.rs`
//!   build-ran pattern. It also names the boundary types
//!   (`InspectTarget` / the presenter's `WorldCamera` / `CELL_PX` / `cell_to_world` /
//!   `ActiveLevel` / `gdtf_battle_sim::{Cell, Level, CellLevel}`) so the chain is
//!   compile-proven reachable.
//!
//! - The picking tests drive `pick_hovered_cell` in a focused headless app: a
//!   `WorldCamera` is SYNTHESIZED at a known transform with a deterministic
//!   orthographic projection (no render pipeline — `Camera.computed` is set in the
//!   test body, mirroring bevy's own `viewport_to_world` unit test), a `Window` +
//!   `PrimaryWindow` is spawned with a known cursor, `ActiveLevel` + the
//!   `BattleInProgress` gate are inserted, then `app.update()` runs the real
//!   systems and the test asserts on `InspectTarget`.
//!
//! - GTW-251 AC1 (picker emits a request matching `InspectTarget`): a probe drains the
//!   presenter-owned `HighlightRequest` buffer AFTER `emit_highlight_request` and
//!   asserts the emitted request equals the resolved `InspectTarget` (`Some(cell)` when
//!   hovered, `None` when off-grid). The DRAWING moved to the presenter (GTW-251), so
//!   the old input-side draw test migrated there (`tests/highlight_draw.rs`).
//!
//! Every `app.world_mut()` / cursor / camera mutation is in a TEST BODY — the
//! accepted headless idiom (`bevy-traps.md` #7 carve-out (a)). No function here
//! takes `&mut World`/`&World`.

mod harness;
mod highlight_emit;
mod resolve;
mod scene_wiring;
mod viewport;
