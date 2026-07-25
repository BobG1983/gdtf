//! The **injury vocabulary** — the typed foundation of the GTW-405 injury tables
//! (`docs/combat/resolution.md` injury tables).
//!
//! When a ganger takes a non-graze, non-fatal wound, the game rolls a named
//! **injury** from a weighted per-`(category, severity)` table and applies its
//! effects for the rest of the battle (carried forward on the roster). THIS slice
//! (GTW-435) is **only the types** — the vocabulary every later slice builds on; it
//! adds NO loader, NO runtime system, NO RNG draw, NO presenter (those are
//! GTW-437 loader / GTW-438 roll / GTW-439 presenter, with the GTW-436 projector
//! reading the deltas).
//!
//! The pieces, by file:
//! - [`StatTarget`] / [`StatKind`] ([`stat_target`]) — the fresh sim stat
//!   discriminant (the eight direct attributes + the eight derived stats) and its
//!   attribute-vs-derived split.
//! - [`InjuryEffect`] + its payload newtypes and per-effect behaviours — RE-HOMED
//!   into the [`crate::effects::injuries`] palette (GTW-550: one file per effect,
//!   each impl-ing [`ApplyInjuryEffect`]); re-exported here so `crate::injuries::*`
//!   paths keep resolving.
//! - [`InjuryName`] / [`PopupText`] / [`LogText`] / [`InspectText`] ([`text`]) — the
//!   name identity and the three routed display texts.
//! - [`InjuryDef`] / [`PostHeal`] ([`def`]) — the authored per-injury record and its
//!   parsed-but-unread post-heal placeholder.
//! - [`InjuryWeighting`] / [`WeightedInjuryEntry`] / [`InjuryWeight`] /
//!   [`WeightedInjuryTable`] ([`weighting`]) — the authored per-category weighting file
//!   and the built per-bucket table.
//! - [`DamageContext`] ([`context`]) — the wound-source axis (ranged / melee / fall) the
//!   weighting tables key on (GTW-452), so the SAME shared pool is weighted per source.
//! - [`RolledInjury`] / [`GainedInjury`] ([`rolled`]) — the in-fold roll verdict and
//!   the persistent ledger entry.
//! - [`InflictedInjuries`] / [`StatDeltaLedger`] / [`StatDeltaSum`] /
//!   [`BleedAfflicted`] ([`ledger`]) — the per-ganger ledger (the SOLE delta source),
//!   its per-stat summed-delta store, and its bleed accrual; [`HandsAvailable`]
//!   ([`hands`]) — the read-derived hand count its hand projection folds.
//! - [`InjuryRegistry`] ([`registry`]) — the name→[`InjuryDef`] map the GTW-437 loader
//!   builds from the loaded `*.injury.ron` files.
//! - [`InjuryTables`] ([`tables`]) — the per-`(category, severity)`
//!   [`WeightedInjuryTable`] map the GTW-437 loader builds from the `*.weighting.ron`
//!   files (canonically sorted, so the roll is enumeration-order-independent).
//!
//! **The modifier-layer invariant** (`docs/combat/resolution.md`): base attributes
//! stay authoritative and are NEVER mutated by an injury; the per-ganger
//! [`InflictedInjuries`] ledger is the SOLE source of every injury stat delta and
//! bleed accrual; the GTW-436 projector re-sums those deltas on every projection
//! (never applied-once), so a `stat.tuning.ron` hot-reload re-applies them by
//! construction rather than wiping them.

pub mod context;
pub mod def;
pub mod hands;
pub mod ledger;
pub mod registry;
pub mod roll;
pub mod rolled;
pub mod stat_target;
pub mod tables;
pub mod text;
pub mod weighting;

// The effect vocabulary + per-effect behaviours live in the GTW-550 palette
// (`crate::effects::injuries`); re-exported here so `crate::injuries::*` and
// `super::*` paths keep resolving unchanged.
pub use context::DamageContext;
pub use def::{InjuryDef, PostHeal};
pub use hands::HandsAvailable;
pub use ledger::{BleedAfflicted, InflictedInjuries, StatDeltaLedger, StatDeltaSum};
pub use registry::InjuryRegistry;
pub use roll::roll_injury;
pub use rolled::{GainedInjury, RolledInjury};
pub use stat_target::{StatKind, StatTarget};
pub use tables::InjuryTables;
pub use text::{InjuryName, InspectText, LogText, PopupText};
pub use weighting::{InjuryWeight, InjuryWeighting, WeightedInjuryEntry, WeightedInjuryTable};

pub use crate::effects::injuries::{
    ApplyBleeding, ApplyDisableHand, ApplyInjuryEffect, ApplyModify, ApplyMovementCostMul,
    BleedAmount, HealError, InjuryEffect, LedgerAccumulators, MovementCostFactor, StatDelta,
};

#[cfg(test)]
mod test;
