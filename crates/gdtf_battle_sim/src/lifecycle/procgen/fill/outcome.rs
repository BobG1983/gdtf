//! [`FilledPlacement`] — the fill pass's result value: the GTW-424 placement plus
//! the packed fill prefabs and the dead-space-as-`default_floor` regions.

use super::super::{
    assembler::{PlacedPrefab, Placement},
    geometry::RegionRect,
};

/// The fully-assembled placement — the GTW-424 [`Placement`] PLUS the GTW-427 fill prefabs
/// and the dead-space-as-`default_floor` regions (C1/C3).
///
/// A named struct (no-bare-types: the filled outcome is a domain value). The GTW-431 emit
/// step pours the spawn placement, every [`fill`](FilledPlacement::fill) prefab, and the
/// [`dead_space`](FilledPlacement::dead_space) regions (as open `default_floor`) into the
/// [`Situation`](crate::situation::Situation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilledPlacement {
    /// The GTW-424 player + enemy spawn placement this fill built on.
    pub(super) placement:  Placement,
    /// The random same-theme `Fill` prefabs the pass packed in, in placement order.
    pub(super) fill:       Vec<PlacedPrefab>,
    /// The leftover dead-space regions the no-fit fallback PADS with open `default_floor`
    /// (the playable area is never shrunk). The GTW-431 emit step floors these.
    pub(super) dead_space: Vec<RegionRect>,
}

impl FilledPlacement {
    /// Build a filled placement from its parts — the GTW-424 assemble placement, the packed
    /// fill prefabs (in placement order), and the leftover dead-space regions (GTW-732: the
    /// [`FillCursor`](super::cursor::FillCursor) finalizes into this).
    #[must_use]
    pub(in crate::lifecycle::procgen) const fn new(
        placement: Placement,
        fill: Vec<PlacedPrefab>,
        dead_space: Vec<RegionRect>,
    ) -> Self {
        Self {
            placement,
            fill,
            dead_space,
        }
    }

    /// The GTW-424 player + enemy spawn placement.
    #[must_use]
    pub const fn placement(&self) -> &Placement {
        &self.placement
    }

    /// The random same-theme fill prefabs (in placement order).
    #[must_use]
    pub fn fill(&self) -> &[PlacedPrefab] {
        &self.fill
    }

    /// The dead-space regions padded with open `default_floor` (the no-fit fallback —
    /// playable area never shrunk).
    #[must_use]
    pub fn dead_space(&self) -> &[RegionRect] {
        &self.dead_space
    }
}
