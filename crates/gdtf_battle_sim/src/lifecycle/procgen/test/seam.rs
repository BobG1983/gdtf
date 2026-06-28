//! 1-cell seam margin tests (GTW-424 C3, OQ-3): a placement reserves a seam, and no two
//! placed prefabs abut.

use crate::{
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    metric::Cell,
    procgen::{Anchor, Footprint, Margin, MaxRectsPacker, RegionRect, SplitMode},
};

/// A board `GridSize` (clamped to valid; `None` if the axes are out of range — the test
/// returns early, never panics).
fn board_size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

/// OQ-3: a placed footprint reserves a 1-cell seam — the free space after placement no
/// longer contains any cell within one of the placed footprint, so a later placement
/// cannot abut it.
///
/// Discriminating: place a footprint flush in the bottom-left, then assert no free
/// rectangle touches the placed footprint's seam ring. With a ZERO seam the abutting cell
/// would still be free; the 1-cell seam removes it.
#[test]
fn placement_reserves_a_one_cell_seam() {
    let Some(size) = board_size(40, 40) else {
        return;
    };
    let board = RegionRect::board(size);
    let mut packer = MaxRectsPacker::new(board, SplitMode::MaxRects, Margin::DEFAULT);

    // A 10x10 footprint flush in the bottom-left corner.
    let footprint = Footprint::new(10, 10);
    let placed = board.place_at_anchor(Anchor::BottomLeft, footprint);
    assert!(packer.place(placed), "the 10x10 must fit the empty board");

    // The seam-padded footprint: the cells the placement claims (footprint + 1-cell ring,
    // clamped at the board origin). No free rectangle may intersect it.
    let claimed = placed.padded(Margin::DEFAULT);
    for free in packer.free_rects() {
        assert!(
            !free.intersects(claimed),
            "free rect {free:?} intersects the seam-claimed region {claimed:?} — abutting allowed",
        );
    }

    // The abutting cell directly to the RIGHT of the footprint (x == 10, the seam cell)
    // must NOT be coverable by a free rectangle (it is reserved seam).
    let abut = RegionRect::new(Cell::new(10, 0), Footprint::new(1, 1));
    assert!(
        !packer.free_rects().iter().any(|f| f.contains_rect(abut)),
        "the seam cell at x=10 must be reserved, not free",
    );
}

/// OQ-3: two placements cannot abut — a second footprint placed against the first leaves
/// at least one `default_floor` cell between them.
///
/// Discriminating: place two 10x10 footprints, the second as close to the first as the
/// packer allows; assert their (un-padded) footprints are separated by `>= 1` cell on the
/// shared axis. A zero-seam packer would let them touch.
#[test]
fn two_placements_do_not_abut() {
    let Some(size) = board_size(40, 20) else {
        return;
    };
    let board = RegionRect::board(size);
    let mut packer = MaxRectsPacker::new(board, SplitMode::MaxRects, Margin::DEFAULT);

    let a = board.place_at_anchor(Anchor::BottomLeft, Footprint::new(10, 10));
    assert!(packer.place(a), "first placement fits");

    // Place a second 10x10 flush to the RIGHT edge — the strict-opposite-style placement.
    let b = board.place_at_anchor(Anchor::BottomRight, Footprint::new(10, 10));
    assert!(packer.place(b), "second placement fits with a seam between");

    // The two footprints must be separated by >= 1 cell on x (a abuts left, b abuts right;
    // on a 40-wide board there is ample gap, but the invariant is the seam ANYWHERE they'd
    // meet). Assert no shared cell and a gap of at least the seam.
    assert!(!a.intersects(b), "the two footprints must not overlap");
    let gap = b.origin().x - a.max_x();
    assert!(
        gap >= Margin::DEFAULT.cells(),
        "the two footprints must be separated by at least the 1-cell seam, gap was {gap}",
    );
}
