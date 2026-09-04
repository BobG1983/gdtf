use bevy::asset::uuid::Uuid;
use gdtf_battle_sim::{
    metric::{Cell, CellLevel, Level},
    terrain::{def::TerrainUuid, facing::TerrainFacing},
};
use serde::Deserialize;

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    editor_map::PaintedPiece,
    mcp::wire::{PaintedMapNet, PaintedRowNet},
};

// A client's own reading of a slot, so a swapped pair of coordinates is caught.
#[derive(Debug, Deserialize)]
struct CellRow {
    x:     i32,
    y:     i32,
    level: u8,
}

// A client's own reading of a cardinal side.
#[derive(Debug, Deserialize, PartialEq, Eq)]
enum FacingRow {
    North,
    East,
    South,
    West,
}

// A client's own reading of one painted slot, with cell, tile and facing all distinct.
#[derive(Debug, Deserialize)]
struct PaintedRow {
    cell:   CellRow,
    tile:   String,
    facing: FacingRow,
}

fn a_tile() -> TerrainUuid {
    TerrainUuid::new(Uuid::from_u128(0x0B0A_4D00))
}

fn a_slot() -> CellLevel {
    CellLevel::new(Cell::new(5, -2), Level::new(3))
}

fn a_row() -> PaintedRowNet {
    PaintedRowNet::from_piece(a_slot(), PaintedPiece::new(a_tile(), TerrainFacing::West))
}

fn read_back(row: &PaintedRowNet) -> PaintedRow {
    let Ok(text) = ron::ser::to_string(row) else {
        unreachable!("a painted row serializes to compact RON");
    };
    match ron::de::from_str::<PaintedRow>(&text) {
        Ok(read) => read,
        Err(fault) => unreachable!("`{text}` reads back as a slot, a tile and a facing: {fault}"),
    }
}

#[test]
fn a_painted_row_round_trips() {
    assert_ron_round_trip(&a_row());
}

#[test]
fn a_painted_row_carries_each_part_on_its_own_field() {
    let row = read_back(&a_row());
    assert_eq!(
        (row.cell.x, row.cell.y, row.cell.level),
        (5, -2, 3),
        "the slot must arrive as the slot the piece occupies, x as x and y as y: {row:?}",
    );
    assert_eq!(
        row.tile,
        (*a_tile()).to_string(),
        "the tile must arrive as the terrain key painted there: {row:?}",
    );
    assert_eq!(
        row.facing,
        FacingRow::West,
        "the facing must arrive as the side the piece is turned to, or a client would redraw \
         every piece pointing north: {row:?}",
    );
}

#[test]
fn an_empty_storey_is_an_empty_list_rather_than_a_missing_one() {
    let empty = PaintedMapNet::new(Vec::new());
    assert!(
        empty.is_empty(),
        "a storey with nothing painted answers no rows: {empty:?}",
    );
    assert_ron_round_trip(&empty);
}

#[test]
fn a_painted_map_round_trips_every_row_it_holds() {
    let map = PaintedMapNet::new(vec![
        a_row(),
        PaintedRowNet::from_piece(
            CellLevel::new(Cell::new(0, 0), Level::new(0)),
            PaintedPiece::new(a_tile(), TerrainFacing::North),
        ),
    ]);
    assert_eq!(map.len(), 2, "the list keeps both rows: {map:?}");
    assert_ron_round_trip(&map);
}

#[test]
fn the_painted_map_types_trace_usable_shapes() {
    assert_schema_is_usable::<PaintedRowNet>("PaintedRowNet");
    assert_schema_is_usable::<PaintedMapNet>("PaintedMapNet");
}
