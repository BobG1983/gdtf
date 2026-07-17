//! GTW-736: the DEV-ONLY QA `net_qa` transport + request router.
//!
//! The `#![cfg(all(debug_assertions, feature = "net_qa"))]` gate (below, after this crate
//! doc so the doc survives a feature-off build — the `procgen_stepper` suite precedent)
//! compiles the whole dir-form suite to an empty crate without the feature (the CI static
//! build) — the `crate::dev::net_qa` module it exercises does not exist there. Two
//! concerns, one file each:
//!
//! - [`routing`] drives the REAL router headless via
//!   [`NetQaPlugin::with_channels`](gdtf_app::test_support::NetQaPlugin) against a
//!   `GdtfTestAppBuilder` app — Hello negotiation, `GetAppFlow` outside battle, and the
//!   `NoBattle` route-time rejection.
//! - [`transport`] drives the REAL loopback listener over a real `TcpStream` — a framed
//!   Hello round-trip, the one-client-at-a-time `Busy` rejection, and the read-timeout
//!   reap.
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
//! - [`deadline`] drives the generic frame-deadline `Timeout` sweep on a live-battle
//!   `BattleAppBuilder` app via the one still-unclaimed queue (`GetOutput`, the T6 drain);
//!   it reuses [`inject_support`]'s harness.
//! - [`affordance`] drives the REAL router (GTW-746) across three state fixtures (no
//!   battle, the menu, a live battle) and asserts the `available` list `GetAppFlow`
//!   advertises agrees with what the router actually accepts / rejects `NoBattle`; it
//!   reuses [`inject_support`]'s live-battle harness and [`start_battle`]'s menu fixture.
#![cfg(all(debug_assertions, feature = "net_qa"))]

mod affordance;
mod deadline;
mod inject;
mod inject_support;
mod routing;
mod snapshot;
mod start_battle;
mod transport;
