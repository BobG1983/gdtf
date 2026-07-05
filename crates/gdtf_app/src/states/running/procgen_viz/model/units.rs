//! The visualizer's board-cell measure newtypes: spans, coordinates, rectangles, and
//! the whole-board extent. Split out of the monolithic `model.rs` (GTW-583); the
//! model rationale lives on the parent `model` module.

use bevy::prelude::*;

/// A quad's WIDTH in board cells (its x span).
///
/// A viz-local newtype over [`u32`] (no-bare-types rule 1: a cell extent is a domain value,
/// not a bare integer). The sim's [`GridWidth`](gdtf_battle_sim::GridWidth) wraps a `u8` coarse
/// grid span — a prefab footprint width comes off a signed [`Footprint`] cast to `u32`, a
/// distinct concept and inner type — so the visualizer mints its own. Private inner + derived
/// [`Deref`]; built through [`new`](QuadCellW::new).
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct QuadCellW(u32);

impl QuadCellW {
    /// Build a quad width from its cell span.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(cells: u32) -> Self {
        Self(cells)
    }
}

/// A quad's HEIGHT in board cells (its y span).
///
/// A viz-local newtype over [`u32`] (no-bare-types rule 3: distinct from [`QuadCellW`] even
/// over the same inner — a width is never a height). Private inner + derived [`Deref`]; built
/// through [`new`](QuadCellH::new).
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct QuadCellH(u32);

impl QuadCellH {
    /// Build a quad height from its cell span.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(cells: u32) -> Self {
        Self(cells)
    }
}

/// The footprint extent (width × height in cells) of one placed prefab quad — the size the
/// quad's label reports and the size its rectangle covers on the board.
///
/// A named newtype (no-bare-types: a quad's cell extent is a domain value, not a bare pair)
/// with [`QuadCellW`] / [`QuadCellH`] leaves. Read through the named
/// [`width`](QuadSize::width) / [`height`](QuadSize::height) accessors.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct QuadSize {
    /// The quad's width in cells.
    width:  QuadCellW,
    /// The quad's height in cells.
    height: QuadCellH,
}

impl QuadSize {
    /// Build a quad size from its width × height in cells.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(
        width: QuadCellW,
        height: QuadCellH,
    ) -> Self {
        Self { width, height }
    }

    /// The quad's width in cells.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn width(self) -> QuadCellW {
        self.width
    }

    /// The quad's height in cells.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn height(self) -> QuadCellH {
        self.height
    }
}

/// The min-corner X cell coordinate of a quad's board rectangle (`>= 0`).
///
/// A viz-local newtype over [`u32`] (no-bare-types rule 1: a board cell coordinate is a domain
/// value). The sim's [`Cell`](gdtf_battle_sim::Cell) wraps a *signed* `IVec2` pair; the packer
/// never produces a negative origin, so the visualizer projects the min-corner into a
/// non-negative `u32` coordinate it owns. Private inner + derived [`Deref`]; built through
/// [`new`](QuadCellX::new).
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct QuadCellX(u32);

impl QuadCellX {
    /// Build a min-corner X cell coordinate.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(cell: u32) -> Self {
        Self(cell)
    }
}

/// The min-corner Y cell coordinate of a quad's board rectangle (`>= 0`).
///
/// A viz-local newtype over [`u32`] (no-bare-types rule 3: distinct from [`QuadCellX`] even
/// over the same inner — an x is never a y). Private inner + derived [`Deref`]; built through
/// [`new`](QuadCellY::new).
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct QuadCellY(u32);

impl QuadCellY {
    /// Build a min-corner Y cell coordinate.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(cell: u32) -> Self {
        Self(cell)
    }
}

/// A cell rectangle on the board — a min-corner cell plus a cell extent — projected from a
/// placed prefab's region so the draw layer can position + size its quad WITHOUT depending on
/// the sim's `RegionRect` (whose inner is private).
///
/// A named struct (no-bare-types: a board placement rectangle is a domain value, not a bare
/// origin+size tuple). All coords are non-negative board cells (the packer never produces a
/// negative origin) — a [`QuadCellX`] / [`QuadCellY`] min-corner plus a [`QuadSize`] extent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct QuadRect {
    /// The min-corner cell x of the rectangle (`>= 0`).
    min_x:  QuadCellX,
    /// The min-corner cell y of the rectangle (`>= 0`).
    min_y:  QuadCellY,
    /// The rectangle's width × height in cells.
    extent: QuadSize,
}

impl QuadRect {
    /// Build a quad rectangle from its min-corner cell + cell extent.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(
        min_x: QuadCellX,
        min_y: QuadCellY,
        extent: QuadSize,
    ) -> Self {
        Self {
            min_x,
            min_y,
            extent,
        }
    }

    /// The min-corner cell x.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn min_x(self) -> QuadCellX {
        self.min_x
    }

    /// The min-corner cell y.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn min_y(self) -> QuadCellY {
        self.min_y
    }

    /// The rectangle's cell extent.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn extent(self) -> QuadSize {
        self.extent
    }
}

/// The whole board's cell extent — the size the single DARK whole-level quad covers (C2).
///
/// A named newtype over [`QuadSize`] (no-bare-types: the board extent is a distinct domain
/// value from a prefab footprint, even over the same shape). The draw layer scales every
/// per-prefab quad's board rectangle against this extent so the quads sit correctly within
/// the dark board quad.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct BoardExtent(QuadSize);

impl BoardExtent {
    /// Build a board extent from its cell width × height.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(extent: QuadSize) -> Self {
        Self(extent)
    }

    /// The board's cell extent.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn size(self) -> QuadSize {
        self.0
    }
}
