//! The top-level protocol envelope — [`QaRequest`] / [`QaResponse`] and their reply
//! payloads (GTW-734).
//!
//! The outermost wire types: every exchange is one [`QaRequest`] in and one
//! [`QaResponse`] out. One concern per file — the [`hello`] handshake
//! ([`ProtocolVersion`] / [`HelloFacts`]),
//! the inject [`receipt`], the [`error`] vocabulary, the [`screenshot`] result, and the
//! [`request`] / [`response`] enums themselves.

pub mod error;
pub mod hello;
pub mod receipt;
pub mod request;
pub mod response;
pub mod screenshot;

pub use error::QaError;
pub use hello::{HelloFacts, ProtocolVersion, ServerNameNet};
pub use receipt::{InjectReceipt, RejectReason};
pub use request::QaRequest;
pub use response::QaResponse;
pub use screenshot::{ScreenshotAfterResult, ScreenshotPathNet, ScreenshotResult};

#[cfg(test)]
mod test;
