use bevy::prelude::Vec2;
use gdtf_battle_presenter::{CELL_PX, cell_to_world};
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};

use crate::picking::{
    hovered::{InspectMode, InspectTarget},
    projection::world_to_cell,
};

#[test]
fn world_to_cell_inverts_cell_to_world() {
    let level = Level::new(0);
    for (cx, cy) in [(0, 0), (1, 2), (12, 7), (59, 59), (30, 0)] {
        let cell = Cell::new(cx, cy);
        let world = cell_to_world(cell, level);
        let back = world_to_cell(world.truncate(), level);
        assert_eq!(
            back,
            Some(CellLevel::new(cell, level)),
            "cell ({cx},{cy}) must round-trip through cell_to_world -> world_to_cell",
        );
    }
}

#[test]
fn world_to_cell_floors_within_a_cell() {
    let level = Level::new(0);
    let interior = Vec2::new(3.5 * CELL_PX, -4.5 * CELL_PX);
    assert_eq!(
        world_to_cell(interior, level),
        Some(CellLevel::new(Cell::new(3, 4), level)),
        "an interior world point must floor into its containing cell",
    );
}

#[test]
fn world_to_cell_off_grid_is_none() {
    let level = Level::new(0);
    assert_eq!(
        world_to_cell(Vec2::new(-CELL_PX, 0.0), level),
        None,
        "a world point left of column 0 must be None",
    );
    assert_eq!(
        world_to_cell(Vec2::new(0.0, -60.0 * CELL_PX), level),
        None,
        "a world point below row 59 must be None",
    );
}

fn sample_cell() -> CellLevel {
    CellLevel::new(Cell::new(7, 3), Level::new(0))
}

#[test]
fn unpinned_effective_is_the_hovered_cell() {
    let cell = sample_cell();
    let target = InspectTarget::new(Some(cell));

    assert_eq!(target.pinned(), None, "a fresh target has no pin");
    assert_eq!(target.hovered(), Some(cell), "the live hovered cell is set");
    assert_eq!(
        target.effective(),
        InspectMode::Hovered(Some(cell)),
        "with no pin, effective() is the live hovered cell",
    );
}

#[test]
fn pin_overrides_effective_but_hover_keeps_tracking() {
    let first = sample_cell();
    let pinned = CellLevel::new(Cell::new(20, 4), Level::new(0));
    let mut target = InspectTarget::new(Some(first));

    target.set_pinned(pinned);
    assert_eq!(target.pinned(), Some(pinned), "the pin is recorded");
    assert_eq!(
        target.effective(),
        InspectMode::Pinned(pinned),
        "a pin makes effective() the pinned cell, not the hovered cell",
    );

    let moved = CellLevel::new(Cell::new(12, 9), Level::new(0));
    assert_ne!(moved, first, "the cursor moved to a new cell");
    assert_ne!(moved, pinned, "the cursor moved off the pinned cell");
    target.set_hovered(Some(moved));
    assert_eq!(
        target.hovered(),
        Some(moved),
        "the live hovered cell keeps tracking the cursor under a pin",
    );
    assert_eq!(
        target.effective(),
        InspectMode::Pinned(pinned),
        "the effective mode is still Pinned while the pin holds",
    );

    target.clear_pin();
    assert_eq!(target.pinned(), None, "the pin is cleared");
    assert_eq!(
        target.effective(),
        InspectMode::Hovered(Some(moved)),
        "unpin resumes hover with the CURRENT live cell (not the stale pre-pin cell)",
    );
}
