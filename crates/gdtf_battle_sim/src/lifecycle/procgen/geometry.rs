//! The cell-rectangle geometry the packer reasons in — [`Footprint`], [`RegionRect`],
//! [`Margin`], and the anchor-flush placement math (GTW-424).
//!
//! Everything here is in CELL units on the ground plane (the sim's coarse-grid metric,
//! `docs/combat/battle-space.md`). The packer never touches sim-unit [`SimPos`] — it
//! places whole-cell footprints, so all arithmetic is integer and exact.

use bevy::math::IVec2;

use super::anchor::Anchor;
use crate::{
    level::{GridSize, GridWidth},
    metric::Cell,
};

/// The 1-cell **seam margin** reserved around every placed prefab (OQ-3 RULED).
///
/// A named newtype over [`u8`] (no-bare-types: a seam width is a domain value, not a bare
/// integer) with a private inner. NO abutting prefabs: every placed footprint reserves at
/// least this many `default_floor` cells of clear space on every side, and that seam
/// lattice is what makes the level connected BY CONSTRUCTION (OQ-4: no doorway carving, no
/// self-repair — the seam lattice alone joins every open cell). The default is one cell
/// ([`Margin::DEFAULT`]); the type exists so the value is named and a future tune is a
/// single edit, not a scattered magic `1`.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Margin(u8);

impl Margin {
    /// The RULED default seam: one `default_floor` cell around every prefab (OQ-3).
    pub const DEFAULT: Self = Self(1);

    /// Build a seam margin from its cell width.
    #[must_use]
    pub const fn new(cells: u8) -> Self {
        Self(cells)
    }

    /// This margin as an `i32` cell count — for the integer rectangle arithmetic the
    /// packer does (insetting a region, padding a placed footprint).
    #[must_use]
    pub const fn cells(self) -> i32 {
        self.0 as i32
    }
}

/// A prefab's **footprint** — its width × height extent on the ground plane, in cells
/// (GTW-424).
///
/// A named newtype over [`IVec2`] (no-bare-types: a placement extent is a domain value,
/// not a bare vector) with a private inner. Derived from a [`GridSize`] via
/// [`Footprint::of`] (a prefab's `size` is its footprint), or built directly for the board
/// extent. The z (storey count) is dropped here — the packer space-packs on the GROUND
/// plane only; vertical extent rides along inside each placed prefab's own slabs/links.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Footprint(IVec2);

impl Footprint {
    /// Build a footprint from its width × height in cells.
    #[must_use]
    pub const fn new(width: i32, height: i32) -> Self {
        Self(IVec2::new(width, height))
    }

    /// The ground-plane footprint of a [`GridSize`] — its width × height (the storey
    /// count is dropped; the packer reasons on the ground plane only).
    #[must_use]
    pub fn of(size: GridSize) -> Self {
        Self::new(i32::from(*size.width()), i32::from(*size.height()))
    }

    /// This footprint's width in cells.
    #[must_use]
    pub const fn width(self) -> i32 {
        self.0.x
    }

    /// This footprint's height in cells.
    #[must_use]
    pub const fn height(self) -> i32 {
        self.0.y
    }

    /// The smaller of the two axes — the side the OQ-5 minimum-size check measures
    /// (a `>= MIN` player-spawn prefab must clear the floor on BOTH axes).
    #[must_use]
    pub const fn min_side(self) -> i32 {
        if self.0.x < self.0.y {
            self.0.x
        } else {
            self.0.y
        }
    }

    /// This footprint's AREA in cells (`width * height`) — the GTW-427 fill pass measures a
    /// prefab against the [`LargePrefabAreaThreshold`](super::tuning::LargePrefabAreaThreshold)
    /// by area. Widened to `i64` so a max-board footprint (60×60) cannot overflow.
    #[must_use]
    pub const fn area(self) -> i64 {
        self.0.x as i64 * self.0.y as i64
    }
}

/// An axis-aligned **rectangle of cells** — an origin (its min-corner cell) plus a
/// [`Footprint`] (GTW-424).
///
/// A named struct (no-bare-types: a placed region is a domain value, not a bare
/// origin+size tuple). The board itself is one [`RegionRect`]
/// ([`RegionRect::board`]); each placed prefab occupies one; the MAXRECTS packer tracks
/// free space as a list of these. The origin is the cell at the rectangle's minimum x AND
/// minimum y corner; the rectangle spans `origin.x .. origin.x + footprint.width` and
/// `origin.y .. origin.y + footprint.height` (a half-open cell range).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionRect {
    /// The minimum-corner cell (min x, min y) of the rectangle.
    origin:    Cell,
    /// The width × height extent in cells.
    footprint: Footprint,
}

impl RegionRect {
    /// Build a region rectangle from its min-corner [`Cell`] and [`Footprint`].
    #[must_use]
    pub const fn new(origin: Cell, footprint: Footprint) -> Self {
        Self { origin, footprint }
    }

    /// The whole board as a region rectangle — origin `(0, 0)`, footprint = the
    /// [`GridSize`]'s ground extent. The packer's root free rectangle.
    #[must_use]
    pub fn board(size: GridSize) -> Self {
        Self::new(Cell::new(0, 0), Footprint::of(size))
    }

    /// The min-corner cell (min x, min y).
    #[must_use]
    pub const fn origin(self) -> Cell {
        self.origin
    }

    /// The width × height extent.
    #[must_use]
    pub const fn footprint(self) -> Footprint {
        self.footprint
    }

    /// The exclusive max x edge (`origin.x + width`) — one past the last cell column.
    #[must_use]
    pub fn max_x(self) -> i32 {
        self.origin.x + self.footprint.width()
    }

    /// The exclusive max y edge (`origin.y + height`) — one past the last cell row.
    #[must_use]
    pub fn max_y(self) -> i32 {
        self.origin.y + self.footprint.height()
    }

    /// Whether this rectangle is non-empty (both axes `> 0`) — the packer prunes empty
    /// free rectangles from its list.
    #[must_use]
    pub const fn is_non_empty(self) -> bool {
        self.footprint.width() > 0 && self.footprint.height() > 0
    }

    /// This rectangle's cell COUNT (`width * height`) — the GTW-427 fill pass sums placed
    /// regions' counts over the board's count to measure coverage against the
    /// [`MinDensityFloor`](super::tuning::MinDensityFloor). Negative extents (an empty
    /// intersection) clamp to `0`.
    #[must_use]
    pub const fn cell_count(self) -> i64 {
        let w = if self.footprint.width() > 0 {
            self.footprint.width()
        } else {
            0
        };
        let h = if self.footprint.height() > 0 {
            self.footprint.height()
        } else {
            0
        };
        w as i64 * h as i64
    }

    /// Whether `other`'s cells are wholly contained in this rectangle (used by the
    /// MAXRECTS prune step to drop free rectangles a placement fully covers).
    #[must_use]
    pub fn contains_rect(self, other: Self) -> bool {
        self.origin.x <= other.origin.x
            && self.origin.y <= other.origin.y
            && other.max_x() <= self.max_x()
            && other.max_y() <= self.max_y()
    }

    /// Whether this rectangle and `other` share any cell — the packer's overlap test
    /// (no two placed prefabs, nor a prefab and a seam, may overlap).
    #[must_use]
    pub fn intersects(self, other: Self) -> bool {
        self.origin.x < other.max_x()
            && other.origin.x < self.max_x()
            && self.origin.y < other.max_y()
            && other.origin.y < self.max_y()
    }

    /// This rectangle grown OUTWARD by `margin` cells on every side — the seam-padded
    /// footprint a placement claims (OQ-3). Used to reserve the 1-cell seam: the packer
    /// removes the PADDED rectangle from free space, so no later prefab can abut.
    ///
    /// The grown rectangle is clamped to non-negative origin — a placement flush against
    /// the board edge has no room to grow outward there, which is fine (the seam only has
    /// to separate prefabs from EACH OTHER; the board boundary is already a wall).
    #[must_use]
    pub fn padded(self, margin: Margin) -> Self {
        let m = margin.cells();
        let ox = (self.origin.x - m).max(0);
        let oy = (self.origin.y - m).max(0);
        let new_origin = Cell::new(ox, oy);
        let new_w = (self.max_x() + m) - ox;
        let new_h = (self.max_y() + m) - oy;
        Self::new(new_origin, Footprint::new(new_w, new_h))
    }

    /// This rectangle clipped to lie wholly within `bounds` — the intersection of the two.
    ///
    /// The seam-padded footprint of a prefab flush against the board edge would extend
    /// PAST the board there (the outward seam has nowhere to go); the board boundary is
    /// already a wall, so the seam is only needed BETWEEN prefabs. Clipping the padded
    /// rectangle to the board before the fit test models that correctly. An empty
    /// intersection yields a zero-extent rectangle (caught by [`is_non_empty`](RegionRect::is_non_empty)).
    #[must_use]
    pub fn clamped_to(self, bounds: Self) -> Self {
        let x0 = self.origin.x.max(bounds.origin.x);
        let y0 = self.origin.y.max(bounds.origin.y);
        let x1 = self.max_x().min(bounds.max_x());
        let y1 = self.max_y().min(bounds.max_y());
        Self::new(
            Cell::new(x0, y0),
            Footprint::new((x1 - x0).max(0), (y1 - y0).max(0)),
        )
    }

    /// Place a `footprint` flush against `anchor` inside this (board) rectangle — the
    /// anchor-to-origin math (C1/C2).
    ///
    /// Corner anchors push the footprint into the named corner; edge-middle anchors push
    /// it against the named edge and CENTRE it on the perpendicular axis. The returned
    /// origin is clamped so the footprint never starts before the board origin (a
    /// footprint wider than the board would otherwise produce a negative origin); a
    /// footprint that does not fit is rejected upstream by the caller (C2), so this only
    /// has to be sane, never panic.
    #[must_use]
    pub fn place_at_anchor(self, anchor: Anchor, footprint: Footprint) -> Self {
        let board_w = self.footprint.width();
        let board_h = self.footprint.height();
        let fw = footprint.width();
        let fh = footprint.height();

        // Left/right/top/bottom-flush coordinates (clamped non-negative for a footprint
        // larger than the board, which the fit check rejects before we reach here).
        let right = (board_w - fw).max(0);
        let top = (board_h - fh).max(0);
        let centre_x = ((board_w - fw) / 2).max(0);
        let centre_y = ((board_h - fh) / 2).max(0);

        let (ox, oy) = match anchor {
            Anchor::TopRight => (right, top),
            Anchor::TopLeft => (0, top),
            Anchor::BottomRight => (right, 0),
            Anchor::BottomLeft => (0, 0),
            Anchor::RightMiddle => (right, centre_y),
            Anchor::LeftMiddle => (0, centre_y),
            Anchor::TopMiddle => (centre_x, top),
            Anchor::BottomMiddle => (centre_x, 0),
        };
        let origin = Cell::new(self.origin.x + ox, self.origin.y + oy);
        Self::new(origin, footprint)
    }
}

/// The minimum side (in cells) a PLAYER-spawn prefab footprint must clear (OQ-5).
///
/// A named newtype over [`GridWidth`] (no-bare-types: the floor on a footprint side is a
/// domain value; it reuses the validated grid-span newtype since it IS a cell span). The
/// RULED default is `~10x10` ([`MinPlayerSide::DEFAULT`] = 10): a player-spawn prefab
/// whose [`Footprint::min_side`] is below this is rejected, so the deployment zone is
/// never a cramped strip. The cap on the player footprint (so the opposite enemy region
/// always fits) is enforced by the packer's fit check against the opposite region, not
/// here.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MinPlayerSide(GridWidth);

impl MinPlayerSide {
    /// The RULED default minimum player-spawn side — `10` cells (OQ-5, `~10x10`).
    pub const DEFAULT: Self = Self(GridWidth::new(10));

    /// Build a minimum-side floor from its cell span.
    #[must_use]
    pub const fn new(cells: GridWidth) -> Self {
        Self(cells)
    }

    /// This floor as an `i32` cell count — for the `Footprint::min_side` comparison.
    #[must_use]
    pub fn cells(self) -> i32 {
        i32::from(*self.0)
    }
}
