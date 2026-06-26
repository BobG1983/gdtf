//! The **injury vocabulary** — the typed foundation of the GTW-405 injury tables
//! (`docs/combat/resolution.md` injury tables).
//!
//! When a ganger takes a non-graze, non-fatal wound, the game rolls a named
//! **injury** from a weighted per-`(body_part, severity)` table and applies its
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
//! - [`InjuryEffect`] / [`StatDelta`] / [`BleedAmount`] ([`effect`]) — one authored
//!   effect of an injury and its two payload newtypes.
//! - [`InjuryName`] / [`PopupText`] / [`LogText`] / [`InspectText`] ([`text`]) — the
//!   name identity and the three routed display texts.
//! - [`InjuryDef`] / [`PostHeal`] ([`def`]) — the authored per-injury record and its
//!   parsed-but-unread post-heal placeholder.
//! - [`InjuryWeighting`] / [`WeightedInjuryEntry`] / [`InjuryWeight`] /
//!   [`WeightedInjuryTable`] ([`weighting`]) — the authored per-part weighting file
//!   and the built per-bucket table.
//! - [`RolledInjury`] / [`GainedInjury`] ([`rolled`]) — the in-fold roll verdict and
//!   the persistent ledger entry.
//! - [`InflictedInjuries`] / [`StatDeltaLedger`] / [`StatDeltaSum`] /
//!   [`BleedAfflicted`] ([`ledger`]) — the per-ganger ledger (the SOLE delta source),
//!   its per-stat summed-delta store, and its bleed accrual.
//! - [`InjuryRegistry`] ([`registry`]) — the name→[`InjuryDef`] map the GTW-437 loader
//!   builds from the loaded `*.injury.ron` files.
//! - [`InjuryTables`] ([`tables`]) — the per-`(body_part, severity)`
//!   [`WeightedInjuryTable`] map the GTW-437 loader builds from the `*.weighting.ron`
//!   files (canonically sorted, so the roll is enumeration-order-independent).
//!
//! **The modifier-layer invariant** (`docs/combat/resolution.md`): base attributes
//! stay authoritative and are NEVER mutated by an injury; the per-ganger
//! [`InflictedInjuries`] ledger is the SOLE source of every injury stat delta and
//! bleed accrual; the GTW-436 projector re-sums those deltas on every projection
//! (never applied-once), so a `stat_tuning.ron` hot-reload re-applies them by
//! construction rather than wiping them.

pub mod def;
pub mod effect;
pub mod ledger;
pub mod registry;
pub mod roll;
pub mod rolled;
pub mod stat_target;
pub mod tables;
pub mod text;
pub mod weighting;

pub use def::{InjuryDef, PostHeal};
pub use effect::{BleedAmount, InjuryEffect, MovementCostFactor, StatDelta};
pub use ledger::{
    BleedAfflicted, HandsAvailable, InflictedInjuries, StatDeltaLedger, StatDeltaSum,
};
pub use registry::InjuryRegistry;
pub use roll::roll_injury;
pub use rolled::{GainedInjury, RolledInjury};
pub use stat_target::{StatKind, StatTarget};
pub use tables::InjuryTables;
pub use text::{InjuryName, InspectText, LogText, PopupText};
pub use weighting::{InjuryWeight, InjuryWeighting, WeightedInjuryEntry, WeightedInjuryTable};

#[cfg(test)]
mod test;
