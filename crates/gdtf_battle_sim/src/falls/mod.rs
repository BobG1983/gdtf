//! Fall damage when a ganger loses floor support under them.

mod damage;
mod message;
mod plugin;
mod resolve;
mod system;

#[cfg(test)]
mod tests;

pub(crate) use damage::{FallImpact, FallWoundEnv, resolve_fall_hit};
pub use message::{FallOccurred, StoreysFallen};
pub use plugin::FallsPlugin;
pub use resolve::{DropLanding, resolve_drop};
pub use system::apply_falls;
