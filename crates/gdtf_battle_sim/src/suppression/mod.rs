//! Suppression from nearby fire: mark, auto-stance, and turn reset.

mod apply;
mod reset;
mod stance;

#[cfg(test)]
mod test;

pub use apply::{SuppressionApplied, apply_suppression};
pub use reset::reset_suppression;
pub use stance::{stance_for_cover_band, suppression_auto_stance};
