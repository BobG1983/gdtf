//! GTW-736: the DEV-ONLY QA `net_qa` transport + request router.
//!
//! The `#![cfg(all(debug_assertions, feature = "net_qa"))]` gate (below, after this crate
//! doc so the doc survives a feature-off build — the `procgen_stepper` suite precedent)
//! compiles the whole dir-form suite to an empty crate without the feature — the
//! `crate::dev::net_qa` module it exercises does not exist there. CI's test step names the
//! feature (GTW-883), so this suite runs there rather than collecting nothing. Two
//! concerns, one file each:
//!
//! - [`routing`] drives the REAL router headless via
//!   [`NetQaPlugin::with_channels`](gdtf_app::test_support::NetQaPlugin) against a
//!   `GdtfTestAppBuilder` app — Hello negotiation, `GetAppFlow` outside battle, and the
//!   `NoBattle` route-time rejection.
//!   (GTW-803 moved the pure-TRANSPORT suite — the framed round-trip over a real
//!   `TcpStream`, the one-client-at-a-time `Busy` rejection and the read-timeout reap — into
//!   the shared `gdtf_net_qa_transport` crate's own `tests/transport/`, beside the code it
//!   drives.)
//! - [`inject`] drives the REAL T4 `apply_injects` pump (GTW-737) on a live-battle
//!   `BattleAppBuilder` app: same-frame drain of a classic + a contextual intent, the
//!   `NotOffered` offer-gate rejection, the fail-closed `UnknownEntity` token rejection,
//!   and the outcome-decoupled `Queued` receipt — its harness lives in [`inject_support`].
//! - [`snapshot`] drives the REAL T5 `build_snapshots` view service (GTW-738) on a
//!   live-battle `BattleAppBuilder` app: the curated `GetBattleState` `BattleView` CONTENT
//!   (ganger cards + indexed fire modes, terrain token handout, fog / selection / turn), the
//!   round-tripping tokens, and the post-Simulate SAME-FRAME consistency an injected intent
//!   proves; it reuses [`inject_support`]'s harness.
//! - [`start_battle`] drives the REAL T9 `drive_start_battle` consumer (GTW-742) on a
//!   menu-resting `GdtfTestAppBuilder` app: a `StartBattle` descends the real machine to
//!   `BattleRunning` with the REQUESTED seed (the `ShotRng::from_root` determinism
//!   fingerprint), and an unknown situation ref is rejected `BadRequest`.
//! - [`output`] drives the REAL T6 outbox pump (`drive_output`, GTW-739) on a live-battle
//!   `BattleAppBuilder` app: a curated act appended to the sim-owned `ActLog` drains
//!   through the real pump as its projected wire `NetEvent`, a second drain is empty, a ring
//!   overflow reports the dropped gap while retaining the newest entries, and an event cap
//!   bounds the batch while the next drain resumes; it reuses [`inject_support`]'s harness.
//! - [`deadline`] (GTW-739 retargeted) asserts that `GetOutput` in a live battle now returns
//!   a real `EventBatch` from the T6 pump the same frame — it no longer falls through to the
//!   frame-deadline `Timeout` sweep; it reuses [`inject_support`]'s harness.
//! - [`affordance`] drives the REAL router (GTW-746) across three state fixtures (no
//!   battle, the menu, a live battle) and asserts the `available` list `GetAppFlow`
//!   advertises agrees with what the router actually accepts / rejects `NoBattle`; it
//!   reuses [`inject_support`]'s live-battle harness and [`start_battle`]'s menu fixture.
//! - [`raw_input`] drives the REAL GTW-783 raw-input arm of `apply_injects` on a live-battle
//!   `BattleAppBuilder` app: an injected keypress folds into `ButtonInput<KeyCode>` through
//!   Bevy's real `keyboard_input_system` (by physical key AND by named keybind action), a
//!   hover sets the primary window's cursor position, and a focus-set points `InputFocus` at
//!   a live entity (a dead / malformed token fail-closed to `UnknownEntity`); it reuses
//!   [`inject_support`]'s harness.
//! - [`screenshot_after`] drives the REAL T15 `claim_screenshot_after` +
//!   `tick_after_shots` systems (GTW-749) on a live-battle `BattleAppBuilder` app: the
//!   embedded intent drains through the SAME input queue a bare `Inject` uses the same
//!   frame it is claimed, a rejected intent takes no capture ever, and an accepted
//!   intent's capture fires exactly `frame_delay` frames after the claim frame.
//! - [`menu`] drives the REAL router + `drive_activate_menu_item` consumer (GTW-787) on a
//!   menu-resting `GdtfTestAppBuilder` app: `GetAppFlow` folds in the spawned main menu's
//!   curated item set, activating the `Battlescape` token leaves the menu via the real
//!   focus-activation path, and a stale token is rejected `StaleToken`; it reuses
//!   [`start_battle`]'s menu fixture and [`inject_support`]'s request helper.
//! - [`focus`] drives the REAL router + `drive_focus_control` consumer (GTW-802) on a
//!   `MinimalPlugins` app resting on the Options screen: `GetAppFlow` folds in the screen's
//!   OWN navigation-graph focusables, a `Step` moves focus through the real
//!   `apply_navigation`, a `Focus` points `InputFocus`, and a malformed / dead /
//!   live-but-UNLISTED token is rejected `StaleToken`.
//! - [`focus_activation`] drives the same path END TO END on the `DefaultPlugins` tier
//!   (GTW-802): activating the enumerated sound-toggle token over the wire flips the real
//!   checkbox + readout, activating Continue leaves the screen for the Main Menu, and a bare
//!   `Activate` clicks whatever holds focus. Both focus suites share [`focus_support`]'s
//!   fixtures.
#![cfg(all(debug_assertions, feature = "net_qa"))]

mod affordance;
mod caught_up;
mod deadline;
mod focus;
mod focus_activation;
mod focus_support;
mod inject;
mod inject_support;
mod menu;
mod output;
mod raw_input;
mod routing;
mod screenshot_after;
mod snapshot;
mod start_battle;
