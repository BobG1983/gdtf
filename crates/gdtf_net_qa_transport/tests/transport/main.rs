//! Transport tests: the REAL loopback listener over a real `TcpStream` (GTW-736; lifted
//! to this crate in GTW-803).
//!
//! These exercise the TRANSPORT (accept, framing, request/reply correlation, the
//! one-client-at-a-time `Busy` gate, the read-timeout reap) with a stand-in host side that
//! is deliberately NOT a router — only a collaborator, so the transport's correlation can
//! be observed. Request ROUTING is the host's concern and is covered by the host's own
//! suite (`gdtf_app/tests/net_qa/routing.rs`).
//!
//! The tests return `Result` and use `?` for the socket / codec I/O (the workspace denies
//! `unwrap`/`expect`/`panic!` even in tests); shape checks use `assert!(matches!(…))`.
//!
//! - [`round_trip`] — a framed Hello round-trip plus the `Busy` rejection of a second
//!   concurrent client.
//! - [`reap`] — the two-sided socket timeout closing an idle client.

mod harness;
mod reap;
mod round_trip;
