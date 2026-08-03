use crate::{
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    metric::Cell,
    procgen::{Anchor, Footprint, Margin, MaxRectsPacker, RegionRect, SplitMode},
};

fn board_size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

#[test]
fn placement_reserves_a_one_cell_margin() {
    let Some(size) = board_size(40, 40) else {
        return;
    };
    let board = RegionRect::board(size);
    let mut packer = MaxRectsPacker::new(board, SplitMode::MaxRects, Margin::DEFAULT);

    let footprint = Footprint::new(10, 10);
    let placed = board.place_at_anchor(Anchor::BottomLeft, footprint);
    assert!(*packer.place(placed), "the 10x10 must fit the empty board");

    let claimed = placed.padded(Margin::DEFAULT);
    for free in packer.free_rects() {
        assert!(
            !*free.intersects(claimed),
            "free rect {free:?} intersects the margin-claimed region {claimed:?} — abutting allowed",
        );
    }

    let abut = RegionRect::new(Cell::new(10, 0), Footprint::new(1, 1));
    assert!(
        !packer.free_rects().iter().any(|f| *f.contains_rect(abut)),
        "the margin cell at x=10 must be reserved, not free",
    );
}

#[test]
fn two_placements_do_not_abut() {
    let Some(size) = board_size(40, 20) else {
        return;
    };
    let board = RegionRect::board(size);
    let mut packer = MaxRectsPacker::new(board, SplitMode::MaxRects, Margin::DEFAULT);

    let a = board.place_at_anchor(Anchor::BottomLeft, Footprint::new(10, 10));
    assert!(*packer.place(a), "first placement fits");

    let b = board.place_at_anchor(Anchor::BottomRight, Footprint::new(10, 10));
    assert!(
        *packer.place(b),
        "second placement fits with a margin between"
    );

    assert!(!*a.intersects(b), "the two footprints must not overlap");
    let gap = b.origin().x - *a.max_x();
    assert!(
        gap >= *Margin::DEFAULT.cells(),
        "the two footprints must be separated by at least the 1-cell margin, gap was {gap}",
    );
}
