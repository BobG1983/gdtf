use bevy::math::IVec2;

use super::anchor::Anchor;
use crate::{
    level::{GridSize, GridWidth},
    metric::{Cell, CellUnit},
};

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellCount(i64);

impl CellCount {
        #[must_use]
    pub const fn new(count: i64) -> Self {
        Self(count)
    }
}

impl std::ops::Add for CellCount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for CellCount {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Margin(u8);

impl Margin {
        pub const DEFAULT: Self = Self(1);

        #[must_use]
    pub const fn new(cells: u8) -> Self {
        Self(cells)
    }

            #[must_use]
    pub const fn cells(self) -> CellUnit {
        CellUnit::new(self.0 as i32)
    }
}

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Footprint(IVec2);

impl Footprint {
        #[must_use]
    pub const fn new(width: i32, height: i32) -> Self {
        Self(IVec2::new(width, height))
    }

            #[must_use]
    pub fn of(size: GridSize) -> Self {
        Self::new(i32::from(*size.width()), i32::from(*size.height()))
    }

        #[must_use]
    pub const fn width(self) -> i32 {
        self.0.x
    }

        #[must_use]
    pub const fn height(self) -> i32 {
        self.0.y
    }

            #[must_use]
    pub const fn min_side(self) -> i32 {
        if self.0.x < self.0.y {
            self.0.x
        } else {
            self.0.y
        }
    }

                #[must_use]
    pub const fn area(self) -> CellCount {
        CellCount::new(self.0.x as i64 * self.0.y as i64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionRect {
        origin:    Cell,
        footprint: Footprint,
}

impl RegionRect {
        #[must_use]
    pub const fn new(origin: Cell, footprint: Footprint) -> Self {
        Self { origin, footprint }
    }

            #[must_use]
    pub fn board(size: GridSize) -> Self {
        Self::new(Cell::new(0, 0), Footprint::of(size))
    }

        #[must_use]
    pub const fn origin(self) -> Cell {
        self.origin
    }

        #[must_use]
    pub const fn footprint(self) -> Footprint {
        self.footprint
    }

        #[must_use]
    pub fn max_x(self) -> CellUnit {
        CellUnit::new(self.origin.x + self.footprint.width())
    }

        #[must_use]
    pub fn max_y(self) -> CellUnit {
        CellUnit::new(self.origin.y + self.footprint.height())
    }

            #[must_use]
    pub const fn is_non_empty(self) -> RectNonEmpty {
        RectNonEmpty::new(self.footprint.width() > 0 && self.footprint.height() > 0)
    }

                    #[must_use]
    pub const fn cell_count(self) -> CellCount {
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
        CellCount::new(w as i64 * h as i64)
    }

            #[must_use]
    pub fn contains_rect(self, other: Self) -> RectContains {
        RectContains::new(
            self.origin.x <= other.origin.x
                && self.origin.y <= other.origin.y
                && other.max_x() <= self.max_x()
                && other.max_y() <= self.max_y(),
        )
    }

            #[must_use]
    pub fn intersects(self, other: Self) -> RectsIntersect {
        RectsIntersect::new(
            self.origin.x < *other.max_x()
                && other.origin.x < *self.max_x()
                && self.origin.y < *other.max_y()
                && other.origin.y < *self.max_y(),
        )
    }

                                #[must_use]
    pub fn padded(self, margin: Margin) -> Self {
        let m = *margin.cells();
        let ox = (self.origin.x - m).max(0);
        let oy = (self.origin.y - m).max(0);
        let new_origin = Cell::new(ox, oy);
        let new_w = (*self.max_x() + m) - ox;
        let new_h = (*self.max_y() + m) - oy;
        Self::new(new_origin, Footprint::new(new_w, new_h))
    }

                                #[must_use]
    pub fn clamped_to(self, bounds: Self) -> Self {
        let x0 = self.origin.x.max(bounds.origin.x);
        let y0 = self.origin.y.max(bounds.origin.y);
        let x1 = *self.max_x().min(bounds.max_x());
        let y1 = *self.max_y().min(bounds.max_y());
        Self::new(
            Cell::new(x0, y0),
            Footprint::new((x1 - x0).max(0), (y1 - y0).max(0)),
        )
    }

                                        #[must_use]
    pub fn place_at_anchor(self, anchor: Anchor, footprint: Footprint) -> Self {
        let board_w = self.footprint.width();
        let board_h = self.footprint.height();
        let fw = footprint.width();
        let fh = footprint.height();

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

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MinPlayerSide(GridWidth);

impl MinPlayerSide {
        pub const DEFAULT: Self = Self(GridWidth::new(10));

        #[must_use]
    pub const fn new(cells: GridWidth) -> Self {
        Self(cells)
    }

        #[must_use]
    pub fn cells(self) -> CellUnit {
        CellUnit::new(i32::from(*self.0))
    }
}

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectNonEmpty(bool);

impl RectNonEmpty {
        #[must_use]
    pub const fn new(non_empty: bool) -> Self {
        Self(non_empty)
    }
}

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectContains(bool);

impl RectContains {
        #[must_use]
    pub const fn new(contains: bool) -> Self {
        Self(contains)
    }
}

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectsIntersect(bool);

impl RectsIntersect {
        #[must_use]
    pub const fn new(intersects: bool) -> Self {
        Self(intersects)
    }
}
