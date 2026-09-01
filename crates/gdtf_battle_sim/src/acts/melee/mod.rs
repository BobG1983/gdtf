//! Melee attacks on gangers and structure cells.

mod cost;
mod dispatch;
mod emit;
mod ganger;
mod queries;
mod snapshot;
mod structure;

pub use cost::{
    CanMelee, MeleeAttacker, MeleeReach, StructureStanding, can_melee, melee_tu_cost,
    structure_stands,
};
pub use dispatch::dispatch_melee;
pub(super) use queries::{MeleeArms, MeleeCombatants, MeleeOutcomes, MeleeRngs, MeleeWorld};
