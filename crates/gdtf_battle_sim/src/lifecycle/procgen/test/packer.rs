use crate::{
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    procgen::{Anchor, Footprint, Margin, MaxRectsPacker, RegionRect, SplitMode},
};

fn board_size(span: u8) -> Option<GridSize> {
    GridSize::new(
        GridWidth::new(span),
        GridHeight::new(span),
        GridLevels::new(1),
    )
    .ok()
}

#[test]
fn fits_then_rejects_oversize() {
    let Some(size) = board_size(20) else {
        return;
    };
    let board = RegionRect::board(size);

    let mut packer = MaxRectsPacker::new(board, SplitMode::MaxRects, Margin::DEFAULT);
    let smaller = board.place_at_anchor(Anchor::BottomLeft, Footprint::new(15, 15));
    assert!(*packer.place(smaller), "a 15x15 must fit a 20x20 board");

    let mut packer2 = MaxRectsPacker::new(board, SplitMode::MaxRects, Margin::DEFAULT);
    let oversize = RegionRect::new(crate::metric::Cell::new(0, 0), Footprint::new(25, 25));
    assert!(
        !*packer2.place(oversize),
        "a 25x25 footprint must not fit a 20x20 board",
    );
    assert_eq!(
        packer2.free_rects().len(),
        1,
        "a rejected placement must leave the free list untouched (still the whole board)",
    );
}

#[test]
fn maxrects_free_list_stays_maximal() {
    let Some(size) = board_size(30) else {
        return;
    };
    let board = RegionRect::board(size);
    let mut packer = MaxRectsPacker::new(board, SplitMode::MaxRects, Margin::DEFAULT);

    let centred = RegionRect::new(crate::metric::Cell::new(10, 10), Footprint::new(8, 8));
    assert!(*packer.place(centred), "centred footprint fits");

    let rects = packer.free_rects();
    for (i, a) in rects.iter().enumerate() {
        for (j, b) in rects.iter().enumerate() {
            if i != j {
                assert!(
                    !(*a.contains_rect(*b) && a != b),
                    "free rect {b:?} is contained in {a:?} — prune invariant broken",
                );
            }
        }
    }
    assert!(!rects.is_empty(), "placing in the middle leaves free space");
}

#[test]
fn guillotine_flag_places_successfully() {
    let Some(size) = board_size(30) else {
        return;
    };
    let board = RegionRect::board(size);
    let mut packer = MaxRectsPacker::new(board, SplitMode::Guillotine, Margin::DEFAULT);
    let placed = board.place_at_anchor(Anchor::BottomLeft, Footprint::new(10, 10));
    assert!(
        *packer.place(placed),
        "the guillotine packer must still place a fitting footprint",
    );
    assert!(
        packer.free_rects().len() <= 2,
        "the guillotine split keeps at most two strips per cut, saw {}",
        packer.free_rects().len(),
    );
}
