//! Resolve RON theme specs into runtime themes.

mod system;
#[cfg(test)]
mod test;

pub use system::{resolve_theme_spec, theme_hot_ron_chain};
