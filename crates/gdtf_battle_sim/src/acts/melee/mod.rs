//! Melee attacks on gangers and structure cells.

mod dispatch;
mod emit;
mod ganger;
mod queries;
mod snapshot;
mod structure;

pub use dispatch::dispatch_melee;
pub(super) use queries::{MeleeArms, MeleeCombatants, MeleeOutcomes, MeleeRngs, MeleeWorld};
