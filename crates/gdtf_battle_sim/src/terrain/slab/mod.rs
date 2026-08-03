//! Slab-HP ledger: the model's single authoritative store of floor/roof **slab**
mod brace_stair_cells;
mod ledger;
mod types;

#[cfg(test)]
mod test;

pub use brace_stair_cells::BraceStairCells;
pub use ledger::SlabLedger;
pub use types::{SlabDamage, SlabDestroyedFlag, SlabEntry, SlabEvent, SlabHp};
