//! The **fall mechanic** (GTW-523, child of GTW-39, parent GTW-17) — the authoritative
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
