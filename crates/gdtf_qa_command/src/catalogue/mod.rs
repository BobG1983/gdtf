//! Building the catalogue a host publishes from that host's own command slice.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`build`] — [`catalogue`], the one walk that produces every published row.

pub mod build;

pub use build::catalogue;
