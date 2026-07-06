//! The SHARED per-frame FCT stack counter (GTW-572 C3) + its reset system.
//!
//! Before GTW-572 every consequence reader kept a LOCAL per-drain stack counter, so two
//! DIFFERENT families popping on ONE cell in ONE frame each took slot `0` and overlapped —
//! the cross-family same-cell defect this resource fixes. The counter is a battle-agnostic
//! presenter resource: [`reset_fct_stacks`] clears it at the top of every frame's draw band,
//! EXPLICITLY ordered before the whole consequence-reader set (`bevy-traps.md` #3 — an
//! unordered reset would race the readers and hand out stale slots), and every generic
//! reader then pulls its next slot from the ONE map — so simultaneous cross-family pops on a
//! cell fan out vertically exactly like same-family ones.

use bevy::{platform::collections::HashMap, prelude::*};
use gdtf_battle_sim::prelude::CellLevel;

use super::text::FctStackIndex;

/// The shared per-frame `(cell, level)` → next-stack-slot counter every consequence-family
/// reader draws from (GTW-572 C3).
///
/// Keyed by the TYPED [`CellLevel`] (never a bare coordinate tuple), valued by the next
/// unclaimed [`FctStackIndex`]. The inner map is private (rule 5): slots are claimed only
/// through [`next`](Self::next) and the frame boundary only through [`reset`](Self::reset).
#[derive(Resource, Debug, Default)]
pub struct FctStackCounter(HashMap<CellLevel, FctStackIndex>);

impl FctStackCounter {
    /// Claim the next stack slot for a pop anchored at `at` — `0` for the first pop on the
    /// cell this frame, one step further down for each subsequent pop (any family).
    pub fn next(&mut self, at: CellLevel) -> FctStackIndex {
        let slot = self.0.entry(at).or_insert(FctStackIndex::BASE);
        let claimed = *slot;
        *slot = FctStackIndex::new(**slot + 1);
        claimed
    }

    /// Clear every per-cell slot — the frame boundary ([`reset_fct_stacks`]).
    pub fn reset(&mut self) {
        self.0.clear();
    }
}

/// `Update` (`PresenterSystems::Overlay`, `.before(ConsequenceFctSystems::Read)`): clear the
/// shared [`FctStackCounter`] so this frame's pops stack from slot `0` per cell.
///
/// The ordering is EXPLICIT (`bevy-traps.md` #3): the reset is configured strictly before
/// the consequence-reader set in
/// [`register_consequence_fct_core`](super::stacked_reader::register_consequence_fct_core),
/// never left ambiguous. Unguarded by any battle gate — the resource is `init_resource`-d
/// on build (always present) and clearing an empty map is free.
pub fn reset_fct_stacks(mut stacks: ResMut<FctStackCounter>) {
    stacks.reset();
}

#[cfg(test)]
mod test {
    use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};

    use super::{FctStackCounter, FctStackIndex};

    /// A cell key at `(x, y, z)` for the counter tests.
    fn at(x: i32, y: i32, z: u8) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(z))
    }

    /// Successive claims on ONE cell hand out ascending slots (0, 1, 2, …) — regardless of
    /// which family claims them — while a DIFFERENT cell starts back at the base slot.
    #[test]
    fn claims_on_one_cell_ascend_and_other_cells_start_at_base() {
        let mut counter = FctStackCounter::default();
        assert_eq!(counter.next(at(3, 4, 0)), FctStackIndex::BASE);
        assert_eq!(counter.next(at(3, 4, 0)), FctStackIndex::new(1));
        assert_eq!(counter.next(at(3, 4, 0)), FctStackIndex::new(2));
        // A different cell (and a different storey of the same cell) each get their own base.
        assert_eq!(counter.next(at(5, 4, 0)), FctStackIndex::BASE);
        assert_eq!(counter.next(at(3, 4, 1)), FctStackIndex::BASE);
    }

    /// A reset returns every cell to the base slot — the per-frame boundary.
    #[test]
    fn a_reset_returns_every_cell_to_the_base_slot() {
        let mut counter = FctStackCounter::default();
        let _ = counter.next(at(3, 4, 0));
        let _ = counter.next(at(3, 4, 0));
        counter.reset();
        assert_eq!(
            counter.next(at(3, 4, 0)),
            FctStackIndex::BASE,
            "after a reset the first claim on a cell is the base slot again",
        );
    }
}
