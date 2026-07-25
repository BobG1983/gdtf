//! The loopback TCP transport — the `std::thread` listener (GTW-736; lifted in GTW-803).
//!
//! Binds [`Ipv4Addr::LOCALHOST`](std::net::Ipv4Addr::LOCALHOST) ONLY (never a routable
//! interface, never configurable — only the port varies) and serves ONE client at a time: a
//! second concurrent connection receives a typed
//! [`Busy`](gdtf_qa_protocol::envelope::QaError::Busy) frame and is closed. The client
//! socket gets two-sided read AND write timeouts so a stuck or idle client cannot hold the
//! single slot forever. Frames follow [`gdtf_qa_protocol::framing`] (`u32` BE length prefix
//! + compact RON) — this transport calls that pure codec, it never reimplements framing.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`bind`] — the loopback bind policy and its ephemeral-port readback.
//! - [`accept`] — the accept loop, its one-client-at-a-time gate, and the `Busy` rejection.
//! - [`serve`] — the per-connection read → decode → hand off → reply conversation.

mod accept;
mod bind;
mod serve;

pub use accept::run_listener;
pub use bind::bind_listener;

#[cfg(test)]
mod test;
