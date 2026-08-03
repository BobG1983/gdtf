//! Typed QA commands and their type-erased form.

/// Object-safe wrapper around a concrete [`QaCommand`].
pub mod erased;
/// JSON Schema text helpers for args and replies.
pub mod schema;
/// The typed command trait hosts implement.
pub mod spec;

pub use erased::ErasedCommand;
pub use spec::QaCommand;

#[cfg(test)]
mod test;
