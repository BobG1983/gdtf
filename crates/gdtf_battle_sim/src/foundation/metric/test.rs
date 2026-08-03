//! from the former inline `#[cfg(test)] mod tests`).

use bevy::math::{IVec2, IVec3, Vec3};

use crate::metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos, cell_center, pos_to_cell};

#[test]
fn metric_consts_match_battle_space_doc() {
    assert_eq!(MAX_LEVELS, 8);
}

#[test]
fn cell_wraps_ivec2_and_derefs() {
    let cell = Cell::new(3, -4);
    assert_eq!(cell.x, 3);
    assert_eq!(cell.y, -4);
    assert_eq!(*cell, IVec2::new(3, -4));
}

#[test]
fn level_wraps_u8_and_derefs() {
    let level = Level::new(5);
    assert_eq!(*level, 5u8);
    assert!(*level < MAX_LEVELS);
}

#[test]
fn cell_level_wraps_ivec3_and_derefs() {
    let key = CellLevel::new(Cell::new(7, 8), Level::new(2));
    assert_eq!(key.x, 7);
    assert_eq!(key.y, 8);
    assert_eq!(key.z, 2);
    assert_eq!(*key, IVec3::new(7, 8, 2));
}

#[test]
fn sim_pos_wraps_vec3_and_derefs() {
    let p = SimPos::new(1.5, 2.5, 3.0);
    assert_eq!(*p, Vec3::new(1.5, 2.5, 3.0));
    assert_eq!(p.x.to_bits(), 1.5_f32.to_bits());
    assert_eq!(p.y.to_bits(), 2.5_f32.to_bits());
    assert_eq!(p.z.to_bits(), 3.0_f32.to_bits());
}

#[test]
fn cell_center_sits_at_half_cell_offset_on_level_floor() {
    let center = cell_center(Cell::new(4, 7), Level::new(2));
    assert_eq!(center.x.to_bits(), 4.5_f32.to_bits());
    assert_eq!(center.y.to_bits(), 7.5_f32.to_bits());
    assert_eq!(center.z.to_bits(), 2.0_f32.to_bits());
}

#[test]
fn cell_center_pos_to_cell_round_trip() {
    let cell = Cell::new(3, 9);
    let level = Level::new(4);
    let (back_cell, back_level) = pos_to_cell(cell_center(cell, level));
    assert_eq!(back_cell, cell);
    assert_eq!(back_level, level);
}

#[test]
fn pos_to_cell_floors_negative_coordinates() {
    let cell = Cell::new(-2, -3);
    let level = Level::new(0);
    let (back_cell, back_level) = pos_to_cell(cell_center(cell, level));
    assert_eq!(back_cell, cell, "negative cell must floor, not round");
    assert_eq!(back_level, level);

    let (mid_cell, _) = pos_to_cell(SimPos::new(-0.1, -0.9, 0.0));
    assert_eq!(
        mid_cell,
        Cell::new(-1, -1),
        "a fractional negative coordinate floors to the lower cell",
    );
}

#[test]
fn pos_to_cell_levels_floor_within_a_storey() {
    let (_, low_in_storey) = pos_to_cell(SimPos::new(0.5, 0.5, 2.0));
    let (_, high_in_storey) = pos_to_cell(SimPos::new(0.5, 0.5, 2.99));
    assert_eq!(low_in_storey, Level::new(2));
    assert_eq!(high_in_storey, Level::new(2));
}

#[test]
fn cell_level_split_inverts_new_across_storeys() {
    for storey in [0, 3, MAX_LEVELS - 1] {
        let cell = Cell::new(11, -4);
        let level = Level::new(storey);
        let key = CellLevel::new(cell, level);
        assert_eq!(
            key.split(),
            (cell, level),
            "split() must invert new() at storey {storey}",
        );
        assert_eq!(key.cell(), cell, "cell() must recover the composed cell");
        assert_eq!(
            key.level(),
            level,
            "level() must recover the composed storey",
        );
    }
}

#[test]
fn cell_level_serde_round_trip_rides_the_accessors() {
    for storey in [0, MAX_LEVELS - 1] {
        let key = CellLevel::new(Cell::new(59, 42), Level::new(storey));
        let written = ron::to_string(&key);
        assert!(written.is_ok(), "CellLevel must serialize: {written:?}");
        let Ok(text) = written else {
            return;
        };
        let back = ron::from_str::<CellLevel>(&text);
        assert_eq!(
            back.ok(),
            Some(key),
            "serialize -> deserialize must be the identity at storey {storey}",
        );
    }
}

#[test]
fn cell_level_deserializes_through_its_typed_constructor() {
    let authored = "(cell: (x: 7, y: 8), level: 3)";
    let parsed = ron::from_str::<CellLevel>(authored);
    assert!(parsed.is_ok(), "CellLevel RON must parse: {parsed:?}");
    let Ok(key) = parsed else {
        return;
    };
    assert_eq!(
        key,
        CellLevel::new(Cell::new(7, 8), Level::new(3)),
        "deserialized CellLevel must equal the typed-constructor result",
    );
    assert_eq!(key.x, 7);
    assert_eq!(key.y, 8);
    assert_eq!(key.z, 3);
}
