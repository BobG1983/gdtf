//! The STEP / AUTO reveal counters: how many leading quads are revealed, and each
//! quad's reveal position in the placement sequence. Split out of the monolithic
//! `model.rs` (GTW-583); the model rationale lives on the parent `model` module.

use bevy::prelude::*;

crate::support_item! {
    /// How many leading quads of the placement sequence are currently REVEALED (`0..=len`).
    ///
    /// A viz-local newtype over [`usize`] (no-bare-types rule 1: a reveal count is a domain
    /// value, not a bare index). STEP advances it by one; AUTO sets it to the sequence length.
    /// Private inner + derived [`Deref`] (so a count reads as a `usize`); built through
    /// [`new`](RevealedCount::new). `Default` (zero revealed) is the starting state. Declared
    /// through `crate::support_item!` so it is at least as public as the
    /// [`ProcgenViz::revealed`](super::viz::ProcgenViz::revealed) accessor that returns it (which widens under `test-support`);
    /// the external test reads it through `Deref` rather than naming it.
    #[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
    struct RevealedCount(usize);
}

impl RevealedCount {
    /// Build a reveal count from its number of revealed quads.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(revealed: usize) -> Self {
        Self(revealed)
    }

    /// The number of revealed quads (the const-context read — [`Deref`] is not const).
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn get(self) -> usize {
        self.0
    }
}

crate::support_item! {
    /// A quad's REVEAL position in the placement sequence — its index (`0` = player, `1` =
    /// enemy, then fill in placement order) the STEP / AUTO reveal count is measured against.
    ///
    /// A viz-local newtype over [`usize`] (no-bare-types rule 1: a reveal position is a domain
    /// value, not a bare index). Distinct from [`RevealedCount`] (rule 3: a position is not a
    /// count) — a quad is shown once the count EXCEEDS its index
    /// ([`is_revealed_within`](RevealIndex::is_revealed_within)). Private inner + derived
    /// [`Deref`]; built through [`new`](RevealIndex::new). Declared through
    /// `crate::support_item!` so it is at least as public as the [`PrefabQuad::index`]
    /// accessor that returns it (which widens under `test-support`); the external test reads it
    /// through `Deref` rather than naming it.
    ///
    /// [`PrefabQuad::index`]: super::super::components::PrefabQuad::index
    #[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct RevealIndex(usize);
}

impl RevealIndex {
    /// Build a reveal index from its position in the placement sequence.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Whether a quad at this index is REVEALED for the given reveal count — true once the
    /// count exceeds the index (`index < revealed`), the draw layer's show/hide predicate (C2).
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn is_revealed_within(
        self,
        revealed: RevealedCount,
    ) -> bool {
        self.0 < revealed.get()
    }
}
