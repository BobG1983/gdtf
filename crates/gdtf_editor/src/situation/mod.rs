//! The authored situation the editor writes back.

mod save;

#[cfg(feature = "mcp")]
pub use save::write_situation_in;
