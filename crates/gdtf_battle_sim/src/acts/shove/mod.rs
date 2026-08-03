//! Deliberate and weapon shove resolution.

mod apply;
mod dispatch;
mod verb;

pub use dispatch::dispatch_shove;
pub use verb::{ShoveOutcome, resolve_shove};
