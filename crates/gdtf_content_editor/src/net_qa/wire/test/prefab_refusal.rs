use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{PaintRefusalNet, SelectTileRefusalNet};

// Both conditions the palette applies, so neither arm can be dropped unnoticed.
const BOTH_CONDITIONS: [SelectTileRefusalNet; 2] = [
    SelectTileRefusalNet::NoTerrainDef,
    SelectTileRefusalNet::NotInTheThemePalette,
];

#[test]
fn both_select_tile_refusals_round_trip() {
    for refusal in BOTH_CONDITIONS {
        assert_ron_round_trip(&refusal);
    }
}

#[test]
fn the_two_select_tile_refusals_read_differently() {
    let [missing_def, off_the_palette] = BOTH_CONDITIONS;
    assert_ne!(
        format!("{missing_def:?}"),
        format!("{off_the_palette:?}"),
        "the palette applies two conditions, and a client that cannot tell which one failed \
         cannot fix the call",
    );
}

#[test]
fn the_paint_refusal_round_trips() {
    assert_ron_round_trip(&PaintRefusalNet::NoSelectedTile);
}

#[test]
fn the_prefab_refusals_trace_usable_shapes() {
    assert_schema_is_usable::<SelectTileRefusalNet>("SelectTileRefusalNet");
    assert_schema_is_usable::<PaintRefusalNet>("PaintRefusalNet");
}
