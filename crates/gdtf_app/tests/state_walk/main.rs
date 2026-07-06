//! GTW-112 regression: the full `AppState` state-walk drives from the default
//! start all the way to [`AppState::Teardown`] without a parent scene
//! auto-advancing past its hosted default child and without the machine
//! wrapping or looping.
//!
//! These are *pin-discriminating* tests: each assertion re-encodes a specific
//! GTW-112 fix so that a regression in a transition target, or a parent scene
//! that auto-advances again, turns the test red rather than letting the walk
//! silently take a wrong path.
//!
//! Since GTW-143, `Load` no longer advances on a frame-1 shortcut — it leaves
//! only once a `GdtfTheme` is present (resolved from assets in the real app).
//! The `MinimalPlugins` walk has no `AssetServer`, so [`walk_app_with_theme`]
//! seeds the theme before the walk; the deep transition graph below is otherwise
//! unchanged.
//!
//! Since GTW-121, `RunningState::Menu` no longer auto-advances either — the menu
//! now spawns a real screen and waits for player action (the button transitions
//! are GTW-122). The `MinimalPlugins` walk has no input, so the helper
//! [`drive_past_menu`] stands in for the player by queuing the `Menu → Options`
//! transition once the walk is resting on `Menu`, after which the rest of the
//! chain (`Options → Game → …`) still auto-advances through its scaffolds.
//!
//! Since GTW-236, the battlescape PERSISTS in `BattleScapeState::BattleRunning`:
//! the placeholder 3-tick turn-budget auto-exit is gone, so the default deep walk
//! now RESTS at `BattleRunning` and only leaves once the explicit
//! `BattleRunningComplete` end-signal marker is inserted (the victory census / flee
//! button — not yet wired — are what insert it in the running game). These tests
//! stand in for that by inserting the marker through the `test_support` surface to
//! drive the chain past `BattleRunning`.
//!
//! The verified sequence (read off the per-scene `move_on` systems, with the
//! menu step now player-driven and the battlescape now persistence-gated):
//! `Init → Load → Intro → Running` at the top level; under `Running`,
//! `Menu →(player)→ Options → Game`; under `Game`, `Setup → HiveScape → BattleScape`;
//! under `BattleScape`,
//! `Generation → AnimateIn → BattleRunning →(rests; explicit BattleRunningComplete)→
//! AnimateOut → AfterMath`; under
//! `AfterMath`, `AnimateIn → DisplayAftermath → AnimateOut`, whose terminal pops
//! all the way out to `RunningState::Quit`, which advances `AppState` to
//! `Teardown`.

mod harness;
mod state_hosting;
mod walk_lifecycle;
