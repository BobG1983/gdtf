//! QA protocol messages: hello handshake, requests, and responses.

pub mod error;
pub mod hello;
pub mod request;
pub mod response;

pub use error::McpSessionError;
pub use hello::{HelloFacts, ProtocolVersion, ServerNameNet};
pub use request::{McpRequest, RunCommand};
pub use response::McpResponse;

#[cfg(test)]
mod test;
