//!   isolated `ApplyX` struct + its `impl ApplyInjuryEffect` + a `#[cfg(test)]` unit
mod apply_effect;
mod bleeding;
mod disable_hand;
mod effect;
mod modify;
mod movement_cost_mul;

#[cfg(test)]
mod tests;

pub use apply_effect::{ApplyInjuryEffect, HandDisabling, HealError, LedgerAccumulators};
pub use bleeding::{ApplyBleeding, BleedAmount};
pub use disable_hand::ApplyDisableHand;
pub use effect::InjuryEffect;
pub use modify::{ApplyModify, StatDelta};
pub use movement_cost_mul::{ApplyMovementCostMul, MovementCostFactor};
