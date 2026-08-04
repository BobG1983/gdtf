//! Max-rects and guillotine free-space packing.

use bevy::prelude::Deref;

use super::geometry::{Footprint, Margin, RegionRect};

/// Whether a candidate fits in free space.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FootprintFits(bool);

impl FootprintFits {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(fits: bool) -> Self {
        Self(fits)
    }
}

/// Whether a placement was accepted.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacementAccepted(bool);

impl PlacementAccepted {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(accepted: bool) -> Self {
        Self(accepted)
    }
}

/// How free space is split after a placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitMode {
    /// Keep all leftover strips (max-rects).
    #[default]
    MaxRects,
    /// Keep one dominant leftover strip (guillotine).
    Guillotine,
}

/// Free-space packer over a board rect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaxRectsPacker {
    board:  RegionRect,
    free:   Vec<RegionRect>,
    split:  SplitMode,
    margin: Margin,
}

impl MaxRectsPacker {
    /// Start with the full board as free space.
    #[must_use]
    pub fn new(board: RegionRect, split: SplitMode, margin: Margin) -> Self {
        Self {
            board,
            free: vec![board],
            split,
            margin,
        }
    }

    /// Current free rects.
    #[must_use]
    pub fn free_rects(&self) -> &[RegionRect] {
        &self.free
    }

    /// Whether the candidate fits inside some free rect (with margin).
    #[must_use]
    pub fn fits(&self, candidate: RegionRect) -> FootprintFits {
        if !*self.board.contains_rect(candidate) {
            return FootprintFits::new(false);
        }
        let padded = candidate.padded(self.margin).clamped_to(self.board);
        FootprintFits::new(self.free.iter().any(|f| *f.contains_rect(padded)))
    }

    /// Place the candidate if it fits; carve free space.
    pub fn place(&mut self, candidate: RegionRect) -> PlacementAccepted {
        if !*self.fits(candidate) {
            return PlacementAccepted::new(false);
        }
        let padded = candidate.padded(self.margin).clamped_to(self.board);
        self.carve(padded);
        PlacementAccepted::new(true)
    }

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
        next.retain(|r| *r.is_non_empty());
        if matches!(self.split, SplitMode::MaxRects) {
            prune_contained(&mut next);
        }
        self.free = next;
    }
}

fn split_maxrects(free: RegionRect, occupied: RegionRect, out: &mut Vec<RegionRect>) {
    let fo = free.origin();
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

fn split_guillotine(free: RegionRect, occupied: RegionRect, out: &mut Vec<RegionRect>) {
    let fo = free.origin();
    let h_leftover =
        (occupied.origin().x - fo.x).max(0) + (*free.max_x() - *occupied.max_x()).max(0);
    let v_leftover =
        (occupied.origin().y - fo.y).max(0) + (*free.max_y() - *occupied.max_y()).max(0);

    if h_leftover >= v_leftover {
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
