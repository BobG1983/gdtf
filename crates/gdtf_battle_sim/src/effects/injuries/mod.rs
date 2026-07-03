//! The **injury-effect palette** (GTW-550 — the injury sibling of the GTW-558
//! attachment palette): the closed
//! [`InjuryEffect`](crate::effects::injuries::InjuryEffect) vocabulary + one isolated
//! behaviour per effect.
//!
//! ## The shape
//!
//! - `apply_effect` — the shared
//!   [`ApplyInjuryEffect`](crate::effects::injuries::ApplyInjuryEffect) trait (the
//!   palette contract: fold / project / heal), the borrowed
//!   [`LedgerAccumulators`](crate::effects::injuries::LedgerAccumulators) fold surface,
//!   and the [`HealError`](crate::effects::injuries::HealError) a non-invertible fold
//!   reports.
//! - `effect` — the closed serde [`InjuryEffect`](crate::effects::injuries::InjuryEffect)
//!   vocabulary (the RON name↔type bridge) + its ONE thin delegation
//!   `impl ApplyInjuryEffect` (each verb forwards through the single `with_behaviour`
//!   match — NO logic).
//! - ONE SELF-CONTAINED FILE PER EFFECT (`modify`, `bleeding`, `disable_hand`,
//!   `movement_cost_mul`), each holding its payload newtype (where it has one) + its
//!   isolated `ApplyX` struct + its `impl ApplyInjuryEffect` + a `#[cfg(test)]` unit
//!   test.
//!
//! ## The discipline (the whole point)
//!
//! Adding a new effect = ONE new per-effect file + ONE
//! [`InjuryEffect`](crate::effects::injuries::InjuryEffect) variant + ONE delegation
//! arm (in `effect`) + ONE `mod` line here. No central logic `match`, no ledger arm,
//! no authoring step scattered across the codebase. The mechanics NEVER match on the
//! effect enum — the ledger's sole mutator
//! ([`InflictedInjuries::gain`](crate::injuries::InflictedInjuries::gain)) and its
//! read-side hand projection invoke the shared trait generically.
//!
//! Dependency direction: this palette depends on the ledger's accumulator newtypes
//! (the stat pieces an effect folds into —
//! [`StatDeltaLedger`](crate::injuries::StatDeltaLedger) /
//! [`BleedAfflicted`](crate::injuries::BleedAfflicted) /
//! [`StatTarget`](crate::injuries::StatTarget)); the ledger MECHANICS (gain / the
//! projections) depend on this palette. STORAGE stays on the ledger — the single
//! `Changed<InflictedInjuries>` source (GTW-550 C3) — never on effect-owned
//! components.

mod apply_effect;
mod bleeding;
mod disable_hand;
mod effect;
mod modify;
mod movement_cost_mul;

#[cfg(test)]
mod tests;

pub use apply_effect::{ApplyInjuryEffect, HealError, LedgerAccumulators};
pub use bleeding::{ApplyBleeding, BleedAmount};
pub use disable_hand::ApplyDisableHand;
pub use effect::InjuryEffect;
pub use modify::{ApplyModify, StatDelta};
pub use movement_cost_mul::{ApplyMovementCostMul, MovementCostFactor};
