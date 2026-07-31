//! Transport tests: the REAL loopback listener over a real `TcpStream` (GTW-736; lifted
//! to this crate in GTW-803).
//!
//! These exercise the TRANSPORT (accept, framing, request/reply correlation, the
//! one-client-at-a-time `Busy` gate, the read-timeout reap) with a stand-in host side that
//! is deliberately NOT a router — only a collaborator, so the transport's correlation can
//! be observed. Request ROUTING is the host's concern and is covered by the host's own
//! suite (`gdtf_app/tests/net_qa/routing.rs`).
//!
//! The `Hello` handshake is answered in the LISTENER thread from the facts its host passed
//! to `run_listener`, and every other frame is refused `NotNegotiated` until it succeeds
//! (GTW-940); the per-connection gate itself is covered by this crate's own unit suite,
//! `src/listener/test/session.rs`.
//!
//! The tests return `Result` and use `?` for the socket / codec I/O (the workspace denies
//! `unwrap`/`expect`/`panic!` even in tests); shape checks use `assert!(matches!(…))`.
//!
//! - [`round_trip`] — the framed handshake, a following request forwarded to the stand-in
//!   host side and correlated with its reply, plus the `Busy` rejection of a second
//!   concurrent client.
//! - [`reap`] — the two-sided socket timeout closing an idle client.

mod harness;
mod reap;
mod round_trip;
