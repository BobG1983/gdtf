//! Shared wire protocol for GDTF QA clients and hosts.

pub mod command;
pub mod framing;
pub mod ids;
pub mod message;

#[cfg(test)]
mod test_support;
