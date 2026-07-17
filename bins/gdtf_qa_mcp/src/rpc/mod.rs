//! The JSON-RPC 2.0 layer — envelope building + method dispatch (GTW-741).
//!
//! [`envelope`] turns a result / error into its wire line; [`dispatch()`] routes an
//! incoming request line to its handler and returns the response line.

pub mod dispatch;
pub mod envelope;

pub use dispatch::dispatch;
