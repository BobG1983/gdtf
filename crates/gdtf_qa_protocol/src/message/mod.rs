//! The top-level protocol message — [`QaRequest`] / [`QaResponse`] and the handshake
//! (GTW-734, frozen by GTW-943).
//!
//! The outermost wire types: every exchange is one [`QaRequest`] in and one
//! [`QaResponse`] out. The module is called `message` because that is what it holds — the
//! request and response types plus the hello handshake, the outer wrapper of one message.
//!
//! One concern per file: the [`hello`] handshake ([`ProtocolVersion`] / [`HelloFacts`]),
//! the [`error`] vocabulary, and the [`request`] / [`response`] enums themselves.

pub mod error;
pub mod hello;
pub mod request;
pub mod response;

pub use error::QaError;
pub use hello::{HelloFacts, ProtocolVersion, ServerNameNet};
pub use request::{QaRequest, RunCommand};
pub use response::QaResponse;

#[cfg(test)]
mod test;
