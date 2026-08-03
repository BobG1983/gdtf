//! Slab HP ledger: structural hit points for floor and roof slabs.

mod brace_stair_cells;
mod ledger;
mod types;

#[cfg(test)]
mod test;

pub use brace_stair_cells::BraceStairCells;
pub use ledger::SlabLedger;
pub use types::{SlabDamage, SlabDestroyedFlag, SlabEntry, SlabEvent, SlabHp};
