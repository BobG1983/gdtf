//! Registered hosts: their names, their values, and the links that reach them.

pub mod name;
pub mod registry;
pub mod runtime;
pub mod set;
pub mod spec;
#[cfg(test)]
pub(crate) mod test_support;

pub use name::HostName;
pub use registry::HostRegistry;
pub use runtime::{HostRuntime, pairs, runtimes};
pub use set::{HostPair, HostSet};
pub use spec::McpHostSpec;
