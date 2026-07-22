//! The DATA-DRIVEN cyclic-act order (GTW-225 / GTW-48 S8 AC8): the fixed authored
//! wrap order for the cyclic acts, DEFINED here so 222b / 222c step it from one
//! source rather than inventing an ad-hoc inline `match` that could diverge.
//!
//! The orders are coordinate-system FACTS (the cardinal compass; the posture
//! ladder), not tuning magnitudes — so they live as authored `const` arrays read
//! by `next_in_cycle`, the single "step-and-wrap" helper.
//! 222b consumes these to build the
//! [`SetStanceRequested`](gdtf_battle_sim::acts::SetStanceRequested) /
//! [`SetFacingRequested`](gdtf_battle_sim::acts::SetFacingRequested) it writes to
//! the intent queue.

mod orders;

#[cfg(test)]
mod test;

pub use orders::{FACING_CYCLE, STANCE_CYCLE, next_facing, next_stance};
