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
//! Version negotiation is enforced HERE rather than by either host (GTW-940): the listener
//! thread answers every [`Hello`](gdtf_qa_protocol::envelope::QaRequest::Hello) from the host's
//! [`HelloFacts`](gdtf_qa_protocol::envelope::HelloFacts), and refuses every other request with
//! [`NotNegotiated`](gdtf_qa_protocol::envelope::QaError::NotNegotiated) until one succeeds — so
//! a client speaking a stale envelope can never reach a host inbox.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`bind`] — the loopback bind policy and its ephemeral-port readback.
//! - [`accept`] — the accept loop, its one-client-at-a-time gate, and the `Busy` rejection.
//! - [`session`] — the per-connection handshake state and the pre-handshake gate.
//! - [`serve`] — the per-connection read → decode → hand off → reply conversation.

mod accept;
mod bind;
mod serve;
mod session;

pub use accept::run_listener;
pub use bind::bind_listener;

#[cfg(test)]
mod test;
