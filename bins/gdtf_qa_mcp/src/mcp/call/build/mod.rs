//! [`build_request`] — a tool's `arguments` → the [`QaRequest`] it maps onto
//! (GTW-741; split into per-concern files per module-layout when GTW-802 added the
//! focus-command parser).
//!
//! ## Members (one concern per file)
//!
//! - [`request`] — the tool → request map itself.
//! - [`intent`] — the `NetIntent` parser (JSON object OR compact-RON string).
//! - [`scalars`] — the scalar argument parsers (cap, name, frame delay, seed, situation,
//!   token).
//! - [`command`] — the two command-carrying parsers (the DEV stepper drive and the focus
//!   drive), each accepting a JSON value OR a compact-RON string.

mod command;
mod intent;
mod request;
mod scalars;

#[cfg(test)]
mod test;

pub use request::build_request;
