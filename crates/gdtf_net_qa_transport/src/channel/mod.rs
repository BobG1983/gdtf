//! The request/response channel plumbing between the listener thread and the host
//! (GTW-736; lifted in GTW-803).
//!
//! The listener thread decodes a [`QaRequest`](gdtf_qa_protocol::envelope::QaRequest) off
//! the socket and hands it to the host side as an [`IncomingRequest`] carrying a
//! [`Responder`] — the one-shot reply channel back to the socket. The [`NetInbox`] resource
//! holds the receiving end.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`responder`] — the one-shot reply channel back to the socket.
//! - [`incoming`] — the request + responder pair that crosses the thread boundary.
//! - [`inbox`] — the Bevy-side receiving end the host drains each frame.

mod inbox;
mod incoming;
mod responder;

pub use inbox::NetInbox;
pub use incoming::IncomingRequest;
pub use responder::Responder;
