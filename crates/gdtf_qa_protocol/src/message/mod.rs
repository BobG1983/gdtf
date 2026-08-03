//! One concern per file: the [`hello`] handshake ([`ProtocolVersion`] / [`HelloFacts`]),
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
