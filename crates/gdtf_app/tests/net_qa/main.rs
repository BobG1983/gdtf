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
#![cfg(all(debug_assertions, feature = "net_qa"))]

mod routing;
mod transport;
