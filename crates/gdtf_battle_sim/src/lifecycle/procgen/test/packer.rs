//! MAXRECTS packer-core tests (GTW-424, OQ-7): fit/no-fit, the maximal-rectangle prune
//! invariant, and the guillotine A/B flag.

use crate::{
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    procgen::{Anchor, Footprint, Margin, MaxRectsPacker, RegionRect, SplitMode},
};

/// A square board `GridSize` (clamped; `None` returns early — never panics).
fn board_size(span: u8) -> Option<GridSize> {
    GridSize::new(
        GridWidth::new(span),
        GridHeight::new(span),
        GridLevels::new(1),
    )
    .ok()
}

/// A footprint that fits the empty board is accepted; one LARGER than the board is
/// rejected with no state change (the free list is untouched).
#[test]
fn fits_then_rejects_oversize() {
    let Some(size) = board_size(20) else {
        return;
    };
    let board = RegionRect::board(size);

    // A 15x15 leaves room — it fits a 20x20 board.
    let mut packer = MaxRectsPacker::new(board, SplitMode::MaxRects, Margin::DEFAULT);
    let smaller = board.place_at_anchor(Anchor::BottomLeft, Footprint::new(15, 15));
    assert!(packer.place(smaller), "a 15x15 must fit a 20x20 board");

    // A 25x25 footprint is LARGER than the 20x20 board — even clamped to the board it
    // cannot be contained, so it is rejected and the free list is left untouched.
    let mut packer2 = MaxRectsPacker::new(board, SplitMode::MaxRects, Margin::DEFAULT);
    let oversize = RegionRect::new(crate::metric::Cell::new(0, 0), Footprint::new(25, 25));
    assert!(
        !packer2.place(oversize),
        "a 25x25 footprint must not fit a 20x20 board",
    );
    assert_eq!(
        packer2.free_rects().len(),
        1,
        "a rejected placement must leave the free list untouched (still the whole board)",
    );
}

/// OQ-7: the MAXRECTS prune keeps only maximal free rectangles — no free rectangle is
/// wholly contained in another after a placement.
///
/// Discriminating: place a footprint in the middle and assert the post-split free list has
/// no contained pair. The split produces overlapping strips; without the prune the list
/// would carry redundant contained rectangles.
#[test]
fn maxrects_free_list_stays_maximal() {
    let Some(size) = board_size(30) else {
        return;
    };
    let board = RegionRect::board(size);
    let mut packer = MaxRectsPacker::new(board, SplitMode::MaxRects, Margin::DEFAULT);

    // A footprint roughly centred (place at the middle of the board).
    let centred = RegionRect::new(crate::metric::Cell::new(10, 10), Footprint::new(8, 8));
    assert!(packer.place(centred), "centred footprint fits");

    let rects = packer.free_rects();
    for (i, a) in rects.iter().enumerate() {
        for (j, b) in rects.iter().enumerate() {
            if i != j {
                assert!(
                    !(a.contains_rect(*b) && a != b),
                    "free rect {b:?} is contained in {a:?} — prune invariant broken",
                );
            }
        }
    }
    assert!(!rects.is_empty(), "placing in the middle leaves free space");
}

/// OQ-7 (A/B flag): the guillotine split is selectable and still places a footprint
/// successfully (it is sparser, not broken).
#[test]
fn guillotine_flag_places_successfully() {
    let Some(size) = board_size(30) else {
        return;
    };
    let board = RegionRect::board(size);
    let mut packer = MaxRectsPacker::new(board, SplitMode::Guillotine, Margin::DEFAULT);
    let placed = board.place_at_anchor(Anchor::BottomLeft, Footprint::new(10, 10));
    assert!(
        packer.place(placed),
        "the guillotine packer must still place a fitting footprint",
    );
    // Guillotine keeps fewer (single-axis) free rectangles than MaxRects would.
    assert!(
        packer.free_rects().len() <= 2,
        "the guillotine split keeps at most two strips per cut, saw {}",
        packer.free_rects().len(),
    );
}
