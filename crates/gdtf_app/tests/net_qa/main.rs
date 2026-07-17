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
//!   `GdtfTestAppBuilder` app — Hello negotiation, `GetAppFlow` outside battle, the
//!   `NoBattle` route-time rejection, and the frame-deadline `Timeout` sweep.
//! - [`transport`] drives the REAL loopback listener over a real `TcpStream` — a framed
//!   Hello round-trip, the one-client-at-a-time `Busy` rejection, and the read-timeout
//!   reap.
//! - [`inject`] drives the REAL T4 `apply_injects` pump (GTW-737) on a live-battle
//!   `BattleAppBuilder` app: same-frame drain of a classic + a contextual intent, the
//!   `NotOffered` offer-gate rejection, the fail-closed `UnknownEntity` token rejection,
//!   and the outcome-decoupled `Queued` receipt — its harness lives in [`inject_support`].
#![cfg(all(debug_assertions, feature = "net_qa"))]

mod inject;
mod inject_support;
mod routing;
mod transport;
