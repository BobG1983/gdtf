//! The **MAXRECTS** free-rectangle packer core (GTW-424, OQ-7).
//!
//! A space-packing packer that tracks free ground-plane space as a list of maximal free
//! [`RegionRect`]s. To place a footprint it finds a free rectangle the (margin-padded)
//! footprint fits in, splits every free rectangle the placement covers into the up-to-four
//! maximal sub-rectangles around it, then prunes any free rectangle wholly contained in
//! another (the MAXRECTS "maximal rectangles" invariant). MAXRECTS is the shipped packer
//! because it is DENSER than a plain guillotine split (OQ-7); a [`SplitMode::Guillotine`]
//! alternative is included behind a flag for A/B comparison.
//!
//! This core is render-free and deterministic: it does NOT draw an RNG of its own (the one
//! RNG draw — the player anchor — happens in [`Anchor::choose`](super::anchor::Anchor::choose)
//! upstream). Given the same free-rectangle list and the same placement requests it
//! produces the same placements every time.

use bevy::prelude::Deref;

use super::geometry::{Footprint, Margin, RegionRect};

/// Whether a (margin-padded) footprint **fits** somewhere in the packer's current free
/// space — the C2 pre-commit fit query ([`MaxRectsPacker::fits`]).
///
/// A named newtype over `bool` (no-bare-types: a fit verdict is a domain fact, not a
/// bare boolean). Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FootprintFits(bool);

impl FootprintFits {
    /// Build a fit verdict from its boolean state.
    #[must_use]
    pub const fn new(fits: bool) -> Self {
        Self(fits)
    }
}

/// Whether a placement was **accepted** — committed to the packer, reserving its
/// margin-padded space ([`MaxRectsPacker::place`]); `false` means the footprint did not
/// fit and the packer is unchanged.
///
/// A named newtype over `bool` (no-bare-types: a placement-accepted verdict is a domain
/// fact, not a bare boolean). Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacementAccepted(bool);

impl PlacementAccepted {
    /// Build a placement-accepted verdict from its boolean state.
    #[must_use]
    pub const fn new(accepted: bool) -> Self {
        Self(accepted)
    }
}

/// How the packer SPLITS a free rectangle when a placement covers part of it (OQ-7).
///
/// A named domain enum (no-bare-types: the split strategy is a domain value). [`MaxRects`](SplitMode::MaxRects)
/// is the shipped, denser strategy (it keeps ALL maximal free sub-rectangles, so later
/// placements can reuse overlapping free space). [`Guillotine`](SplitMode::Guillotine) is
/// the optional A/B alternative behind this flag (a single horizontal-or-vertical cut, no
/// overlap, simpler but sparser) — included per OQ-7's optional clause for comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitMode {
    /// The shipped MAXRECTS split: keep every maximal free sub-rectangle around the
    /// placement, then prune contained rectangles. Denser packing.
    #[default]
    MaxRects,
    /// The optional guillotine split (A/B flag): one axis cut, no overlapping free
    /// rectangles. Simpler, sparser.
    Guillotine,
}

/// The MAXRECTS free-rectangle **packer** — the GTW-424 space-packing core (OQ-7).
///
/// A named struct (no-bare-types: the packer state is a domain value, not a bare
/// `Vec<RegionRect>`) holding the current list of maximal free rectangles + the chosen
/// [`SplitMode`]. Built over the whole board ([`MaxRectsPacker::new`]); each
/// [`place`](MaxRectsPacker::place) consumes the (margin-padded) footprint's space and
/// returns where the UN-padded prefab sits. The margin (OQ-3) is reserved by removing the
/// PADDED rectangle from free space, so no later prefab can abut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaxRectsPacker {
    /// The board the packer fills — the margin-padded footprint is clipped to this before
    /// the fit test, so a prefab flush against a board EDGE is not rejected for an outward
    /// margin that has nowhere to go (the board boundary is already a wall).
    board:  RegionRect,
    /// The current maximal free rectangles. Invariant: none is contained in another
    /// (`MaxRects`) and none is empty.
    free:   Vec<RegionRect>,
    /// The split strategy (shipped MAXRECTS or the A/B guillotine flag).
    split:  SplitMode,
    /// The margin reserved around every placement (OQ-3).
    margin: Margin,
}

impl MaxRectsPacker {
    /// Build a packer over the whole `board` rectangle with a given split mode and margin.
    ///
    /// The single free rectangle is the board itself; placements carve it down. The
    /// `margin` is the [`Margin`] reserved around every placement (OQ-3 — pass
    /// [`Margin::DEFAULT`] for the RULED 1-cell margin).
    #[must_use]
    pub fn new(board: RegionRect, split: SplitMode, margin: Margin) -> Self {
        Self {
            board,
            free: vec![board],
            split,
            margin,
        }
    }

    /// The current free rectangles (read-only) — for the assembler's region-fit query and
    /// the unit tests' invariant checks.
    #[must_use]
    pub fn free_rects(&self) -> &[RegionRect] {
        &self.free
    }

    /// Whether a footprint (PLUS its margin) fits anywhere in the current free space at the
    /// given preferred origin region — used to test C2 fit before committing.
    ///
    /// Tests the margin-PADDED footprint against the free list: a placement is only legal if
    /// its padded extent is wholly inside some free rectangle, which guarantees the 1-cell
    /// margin to every previously placed prefab (OQ-3).
    #[must_use]
    pub fn fits(&self, candidate: RegionRect) -> FootprintFits {
        // The footprint itself must lie inside the board (an oversize footprint clamped to
        // the board would otherwise spuriously "fit"); only the outward MARGIN ring is
        // allowed to be clipped at the board boundary (the edge is already a wall).
        if !*self.board.contains_rect(candidate) {
            return FootprintFits::new(false);
        }
        let padded = candidate.padded(self.margin).clamped_to(self.board);
        FootprintFits::new(self.free.iter().any(|f| *f.contains_rect(padded)))
    }

    /// Commit a placement at `candidate` (the UN-padded prefab footprint) — reserving its
    /// margin-padded extent in the free space (OQ-3) — and return whether it was accepted.
    ///
    /// Returns `false` (no state change) if the padded footprint does not fit any free
    /// rectangle — the caller then fails closed with a [`PackingError`](super::error::PackingError).
    /// On success the padded rectangle is removed from every free rectangle it intersects
    /// (split per [`SplitMode`]), then contained rectangles are pruned (`MaxRects`).
    pub fn place(&mut self, candidate: RegionRect) -> PlacementAccepted {
        if !*self.fits(candidate) {
            return PlacementAccepted::new(false);
        }
        let padded = candidate.padded(self.margin).clamped_to(self.board);
        self.carve(padded);
        PlacementAccepted::new(true)
    }

    /// Remove `occupied` from every free rectangle it intersects, replacing each with the
    /// sub-rectangles around `occupied` per the [`SplitMode`], then prune (`MaxRects`).
    fn carve(&mut self, occupied: RegionRect) {
        let mut next: Vec<RegionRect> = Vec::with_capacity(self.free.len() * 4);
        for free in std::mem::take(&mut self.free) {
            if *free.intersects(occupied) {
                match self.split {
                    SplitMode::MaxRects => split_maxrects(free, occupied, &mut next),
                    SplitMode::Guillotine => split_guillotine(free, occupied, &mut next),
                }
            } else {
                next.push(free);
            }
        }
        // Drop empties.
        next.retain(|r| *r.is_non_empty());
        if matches!(self.split, SplitMode::MaxRects) {
            prune_contained(&mut next);
        }
        self.free = next;
    }
}

/// Split `free` around `occupied` into the up-to-four MAXIMAL free sub-rectangles (left,
/// right, below, above), pushing the non-empty ones to `out` (the MAXRECTS split).
///
/// Each sub-rectangle spans the FULL extent of `free` on its non-cut axis (so it is
/// maximal — overlapping sub-rectangles are the source of MAXRECTS' density). Callers
/// prune contained rectangles afterwards.
fn split_maxrects(free: RegionRect, occupied: RegionRect, out: &mut Vec<RegionRect>) {
    let fo = free.origin();
    // Left strip: free.left .. occupied.left, full free height.
    if occupied.origin().x > fo.x {
        out.push(RegionRect::new(
            fo,
            Footprint::new(occupied.origin().x - fo.x, free.footprint().height()),
        ));
    }
    // Right strip: occupied.right .. free.right, full free height.
    if occupied.max_x() < free.max_x() {
        out.push(RegionRect::new(
            crate::metric::Cell::new(*occupied.max_x(), fo.y),
            Footprint::new(*free.max_x() - *occupied.max_x(), free.footprint().height()),
        ));
    }
    // Bottom strip: free.bottom .. occupied.bottom, full free width.
    if occupied.origin().y > fo.y {
        out.push(RegionRect::new(
            fo,
            Footprint::new(free.footprint().width(), occupied.origin().y - fo.y),
        ));
    }
    // Top strip: occupied.top .. free.top, full free width.
    if occupied.max_y() < free.max_y() {
        out.push(RegionRect::new(
            crate::metric::Cell::new(fo.x, *occupied.max_y()),
            Footprint::new(free.footprint().width(), *free.max_y() - *occupied.max_y()),
        ));
    }
}

/// Split `free` around `occupied` with a single GUILLOTINE cut (the A/B alternative,
/// OQ-7) — the wider remaining gap chooses the cut axis, producing two NON-overlapping
/// sub-rectangles. Simpler and sparser than [`split_maxrects`].
fn split_guillotine(free: RegionRect, occupied: RegionRect, out: &mut Vec<RegionRect>) {
    let fo = free.origin();
    // Horizontal leftover (left + right strips of the occupied band) vs vertical leftover.
    let h_leftover =
        (occupied.origin().x - fo.x).max(0) + (*free.max_x() - *occupied.max_x()).max(0);
    let v_leftover =
        (occupied.origin().y - fo.y).max(0) + (*free.max_y() - *occupied.max_y()).max(0);

    if h_leftover >= v_leftover {
        // Cut horizontally: keep full-height left & right strips, then a single
        // full-width remainder above+below is NOT kept (guillotine: one axis only).
        if occupied.origin().x > fo.x {
            out.push(RegionRect::new(
                fo,
                Footprint::new(occupied.origin().x - fo.x, free.footprint().height()),
            ));
        }
        if occupied.max_x() < free.max_x() {
            out.push(RegionRect::new(
                crate::metric::Cell::new(*occupied.max_x(), fo.y),
                Footprint::new(*free.max_x() - *occupied.max_x(), free.footprint().height()),
            ));
        }
    } else {
        if occupied.origin().y > fo.y {
            out.push(RegionRect::new(
                fo,
                Footprint::new(free.footprint().width(), occupied.origin().y - fo.y),
            ));
        }
        if occupied.max_y() < free.max_y() {
            out.push(RegionRect::new(
                crate::metric::Cell::new(fo.x, *occupied.max_y()),
                Footprint::new(free.footprint().width(), *free.max_y() - *occupied.max_y()),
            ));
        }
    }
}

/// Drop every free rectangle wholly contained in another — the MAXRECTS "keep only
/// maximal rectangles" prune (the split step produces overlapping sub-rectangles; this
/// removes the redundant ones so the list stays small and maximal).
fn prune_contained(rects: &mut Vec<RegionRect>) {
    let mut i = 0;
    while i < rects.len() {
        let mut contained = false;
        for (j, other) in rects.iter().enumerate() {
            if i != j && *other.contains_rect(rects[i]) && (rects[i] != *other || j < i) {
                contained = true;
                break;
            }
        }
        if contained {
            rects.swap_remove(i);
        } else {
            i += 1;
        }
    }
}
