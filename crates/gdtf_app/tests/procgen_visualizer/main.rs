//! GTW-434: headless behavioral tests for the DEV-ONLY procgen STEP/AUTO visualizer.
//!
//! These run on the `MinimalPlugins` [`GdtfTestAppBuilder`] (the real state stack, `UiPlugin`,
//! and — in a debug build — the real `ProcgenVizScenePlugin` wired through `ScenesPlugin`).
//! They seed the persistent `Load` resources the visualizer reads (a theme, a theme+size-only
//! `LoadedSituation`, a [`PrefabRegistry`] with a player + enemy v2 prefab under the
//! situation's [`ThemeUuid`], and a FIXED `BattleSeed` so the assembled level is reproducible),
//! drive into [`RunningState::DebugProcgenVisualizer`](gdtf_app::test_support::RunningState), and
//! assert on the WORLD + the real visualizer model / entities — never on rendering (the
//! screenshot, C5, is the QA stage).
//!
//! GTW-492 (T07b): the visualizer drives the UUID-keyed v2 procgen pipeline, so the fixture
//! seeds a [`PrefabRegistry`] of [`Prefab`] (keyed by the situation's [`ThemeUuid`]).
//!
//! Coverage (C1/C2/C3):
//!
//! - [`step_reveals_one_more_quad`] — a press on the STEP button advances the model's revealed
//!   count by EXACTLY one (C1).
//! - [`auto_reveals_every_quad`] — a press on the AUTO button reveals the WHOLE placement
//!   sequence at once (C1).
//! - [`revealed_quads_carry_role_tints`] — after AUTO, the revealed quad entities carry the
//!   right tints: index 0 (player) = green, index 1 (enemy) = red, the rest = neutral (C3).
//!
//! The whole visualizer is `#[cfg(debug_assertions)]`-gated; these tests run under the dev /
//! gate suite (debug), where the feature is compiled in.

#![cfg(debug_assertions)]

mod config_form;
mod form;
mod generate_drive;
mod harness;
mod reveal;
