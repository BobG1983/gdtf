//! GTW-228 / GTW-265 / GTW-267 (GTW-48 S9 / 222c): headless integration tests for the
//! themed UI action-bar — the `gdtf_app`-side button surface over the SAME 222a
//! act-intent seam the keyboard surface writes, plus the GTW-267 Stance and GTW-265 Mode
//! 3-toggle sub-panels that REPLACED the blind stance cycle and the fire-mode popup
//! picker.
//!
//! The bar lives in `gdtf_app`'s battlescape and is driven here through the REAL
//! stack: a `GdtfTestAppBuilder` headless walk to `BattleScapeState::BattleRunning`
//! (the GTW-221 / `battle_running_driver.rs` battlescape-walk precedent) spawns the
//! real action-bar (its `OnEnter(BattleRunning)` `spawn_action_bar` runs with the
//! injected `default_theme()`), and the tests synthesize `Interaction = Pressed`
//! (`Changed`) on a REAL spawned button — the exact swap a real mouse click drives via
//! `ui_focus_system` (`bevy-traps.md` #6) — then assert the act resolves identically to
//! the equivalent key/intent surface:
//!
//! - AC1 — the bar spawns N markered, interactive `Button`s in the live battle and is
//!   despawned outside it.
//! - AC3 — an aim button press writes the SAME `*Requested` the equivalent 222a intent
//!   does, byte-for-byte (the `acts.rs` AC5 parity idiom); the level buttons mutate
//!   `ActiveLevel` like the level intent.
//! - GTW-267 — the Stance sub-panel: selecting a ganger marks its current stance toggle
//!   `ActiveButton`; pressing Prone direct-sets stance Prone (a `SetStanceRequested`) and
//!   the active mark moves; the three are mutually exclusive.
//! - GTW-265 / GTW-284 — the Mode sub-panel: the THREE FIXED mode toggles are spawned once
//!   and MUTATED in place (GTW-284: never despawned/respawned). A Single+Burst weapon shows
//!   the Single + Burst toggles `Visible` and Full `Hidden`; selecting a ganger marks its
//!   active mode toggle `ActiveButton`; clicking Burst sets `SelectedFireMode` to that
//!   weapon's burst spec and the active mark moves. A weapon change keeps the toggle entity
//!   ids STABLE and only flips their `Visibility`.
//! - AC5 — the DEFERRED reload / end-turn buttons carry `DisabledButton` and emit NO
//!   intent under a synthesized press.
//! - AC6 — with NO `SelectedShooter`, an act-button press is a no-op (no message, no
//!   panic).
//! - AC7 — each button is a `bevy_ui` `Button` + `Node` tree, NOT a
//!   `WORLD_RENDER_LAYER` sprite — it routes to the GTW-120 UI camera.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.
//! Reaching a live battle + driving the GUI itself is the post-gate QA carve-out
//! (in-engine evidence); these synthesized-`Interaction` parity tests are the strong
//! evidence.

mod armed_mode_visibility;
mod bar_scaffold;
mod end_turn_flee;
mod harness;
mod level_buttons;
mod mode_costs;
mod mode_picker;
mod probes;
mod stance_panel;
