//! Line-of-sight probes and engagement visibility checks.

mod engagement;
mod probe;

#[cfg(test)]
mod test;

pub use engagement::{CanSee, can_see};
pub use probe::{Observer, PeekOffset, Sighted, Target, has_los, has_los_peeking};
