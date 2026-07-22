//! The fill cursor's **state-machine vocabulary** (GTW-732) — the per-step yield, the sub-pass
//! enum, and the scatter slot bookkeeping the [`driver`](super::driver) steps through.

use bevy::prelude::Deref;

/// Whether one fill `step` placed a prefab, or the fill is exhausted — the yield the
/// `ProcgenCursor`'s `Filling` phase reads.
///
/// A named domain enum (no-bare-types: a per-step fill outcome is a domain value).
pub(in crate::lifecycle::procgen) enum FillStep {
    /// This step placed exactly one fill prefab; more may remain.
    Placed,
    /// No sub-pass can place any more prefab — the fill is finished.
    Exhausted,
}

/// The total remaining scatter SLOTS the dead-rect scatter sub-pass may still draw against —
/// the `>= 4x4` dead-rect count times the per-rect cap `k`, decremented per draw (GTW-732).
///
/// A named newtype over `usize` (no-bare-types: a remaining-slot count is a domain quantity,
/// distinct from the per-rect `ScatterCount`). Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ScatterSlots(usize);

impl ScatterSlots {
    /// Wrap a remaining-slot count.
    pub(super) const fn new(count: usize) -> Self {
        Self(count)
    }
}

/// The scatter sub-pass's running state — the remaining slots (the old `scatter_dead_rects`
/// only ever used the dead-rect COUNT, so count times `k` is all the cursor must carry).
pub(super) struct ScatterState {
    /// The scatter slots still to draw against.
    pub(super) slots_remaining: ScatterSlots,
}

/// Which fill sub-pass the cursor is running — large fill, then small fill, then dead-rect
/// scatter (the old whole-stage order), plus the terminal `Exhausted`.
pub(super) enum FillPass {
    /// Place large fill prefabs (footprint area `>=` the large threshold).
    Large,
    /// Place the remaining (sub-threshold) fill prefabs.
    Small,
    /// Scatter up to `k` micro-pieces into each `>= 4x4` dead rect.
    Scatter(ScatterState),
    /// Nothing more fits — the fill is finished.
    Exhausted,
}

/// Which bucket a `bucket_step` draws from.
pub(super) enum BucketKind {
    /// The large-prefab bucket.
    Large,
    /// The small-prefab bucket.
    Small,
}

/// Whether a sub-pass iteration placed a prefab, or the sub-pass is done (a private helper
/// outcome, distinct from the public [`FillStep`]).
pub(super) enum SubPassStep {
    /// This iteration placed one prefab.
    Placed,
    /// This sub-pass has nothing more to place.
    Done,
}
