//! Shared wire protocol for MCP QA clients and hosts.

pub mod command;
pub mod framing;
pub mod ids;
pub mod message;
pub mod ports;
pub mod timeouts;

#[cfg(test)]
mod test_support;
