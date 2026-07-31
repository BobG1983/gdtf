//! The two traits one command rides on, plus the one place a schema is derived.
//!
//! [`QaCommand`] is the TYPED trait an implementer writes; [`ErasedCommand`] is its
//! object-safe projection, with exactly one (blanket) implementation, so a host's whole
//! command set is one slice a catalogue walk and a registration walk can share.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`spec`] — the [`QaCommand`] trait.
//! - [`erased`] — the [`ErasedCommand`] trait and its blanket impl.
//! - [`schema`] — `schema_text::<T>()`, the single derivation point.

pub mod erased;
pub mod schema;
pub mod spec;

pub use erased::ErasedCommand;
pub use spec::QaCommand;

#[cfg(test)]
mod test;
