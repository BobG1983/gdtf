//! JSON-RPC request dispatch over stdio lines.

pub mod dispatch;
mod envelope;

pub use dispatch::dispatch;
