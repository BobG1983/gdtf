use gdtf_battle_sim::level::{GridHeight, GridLevels, GridSize, GridWidth};
use serde::Deserialize;

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{
    EditorGridSizeNet,
    grid::{EditorGridHeightNet, EditorGridLevelsNet, EditorGridWidthNet},
};

// A client's own reading of the grid extent, with all three spans distinct.
#[derive(Debug, Deserialize)]
struct GridRow {
    width:  u8,
    height: u8,
    levels: u8,
}

fn small_grid() -> GridSize {
    match GridSize::new(GridWidth::new(6), GridHeight::new(9), GridLevels::new(3)) {
        Ok(size) => size,
        Err(fault) => unreachable!("6x9x3 sits inside every grid bound: {fault}"),
    }
}

fn read_back(size: GridSize) -> GridRow {
    let Ok(text) = ron::ser::to_string(&EditorGridSizeNet::from_size(size)) else {
        unreachable!("a grid size serializes to compact RON");
    };
    match ron::de::from_str::<GridRow>(&text) {
        Ok(row) => row,
        Err(fault) => unreachable!("`{text}` reads back as three named spans: {fault}"),
    }
}

#[test]
fn each_grid_span_round_trips() {
    assert_ron_round_trip(&EditorGridWidthNet::new(6));
    assert_ron_round_trip(&EditorGridHeightNet::new(9));
    assert_ron_round_trip(&EditorGridLevelsNet::new(3));
}

#[test]
fn a_grid_size_round_trips() {
    assert_ron_round_trip(&EditorGridSizeNet::from_size(small_grid()));
}

#[test]
fn a_grid_size_carries_each_span_on_its_own_axis() {
    let size = small_grid();
    let row = read_back(size);
    assert_eq!(
        row.width,
        *size.width(),
        "width must arrive as width. The three spans are distinct here, so a swapped pair \
         reads as a valid grid of the wrong shape: {row:?}",
    );
    assert_eq!(
        row.height,
        *size.height(),
        "height must arrive as height: {row:?}"
    );
    assert_eq!(
        row.levels,
        *size.levels(),
        "levels must arrive as levels: {row:?}"
    );
}

#[test]
fn the_grid_types_trace_usable_shapes() {
    assert_schema_is_usable::<EditorGridWidthNet>("EditorGridWidthNet");
    assert_schema_is_usable::<EditorGridHeightNet>("EditorGridHeightNet");
    assert_schema_is_usable::<EditorGridLevelsNet>("EditorGridLevelsNet");
    assert_schema_is_usable::<EditorGridSizeNet>("EditorGridSizeNet");
}
