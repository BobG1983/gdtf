//! Shared wire protocol for GDTF QA clients and hosts.
//!
//! Framing (length-prefixed RON), message shapes, command catalogue types, and wire ids.

pub mod command;
pub mod framing;
pub mod ids;
pub mod message;

#[cfg(test)]
mod test_support;
