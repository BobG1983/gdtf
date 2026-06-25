//! Relocated unit tests for the cubic-voxel metric (GTW-201 wave 22 — moved verbatim
//! from the former inline `#[cfg(test)] mod tests`).

use bevy::math::{IVec2, IVec3, Vec3};

use crate::metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos, cell_center, pos_to_cell};

#[test]
fn metric_consts_match_battle_space_doc() {
    // Pinned to docs/combat/battle-space.md: the 60×60×8 grid → 8 levels.
    // This is a coordinate-system FACT (not tunable), so it stays pinned —
    // unlike the band-edge level-fractions, which are tuning data.
    assert_eq!(MAX_LEVELS, 8);
}

#[test]
fn cell_wraps_ivec2_and_derefs() {
    let cell = Cell::new(3, -4);
    // Deref reaches the inner IVec2's fields.
    assert_eq!(cell.x, 3);
    assert_eq!(cell.y, -4);
    assert_eq!(*cell, IVec2::new(3, -4));
}

#[test]
fn level_wraps_u8_and_derefs() {
    let level = Level::new(5);
    // Deref reaches the inner u8.
    assert_eq!(*level, 5u8);
    assert!(*level < MAX_LEVELS);
}

#[test]
fn cell_level_wraps_ivec3_and_derefs() {
    let key = CellLevel::new(Cell::new(7, 8), Level::new(2));
    assert_eq!(key.x, 7);
    assert_eq!(key.y, 8);
    // z is the storey index, not a continuous height.
    assert_eq!(key.z, 2);
    assert_eq!(*key, IVec3::new(7, 8, 2));
}

#[test]
fn sim_pos_wraps_vec3_and_derefs() {
    let p = SimPos::new(1.5, 2.5, 3.0);
    // Deref reaches the inner Vec3 (its `glam` PartialEq compares the whole
    // vector, so this checks the inner type without raw `f32 ==`).
    assert_eq!(*p, Vec3::new(1.5, 2.5, 3.0));
    // Per-axis pin by bit pattern (exactly representable sim-unit values).
    assert_eq!(p.x.to_bits(), 1.5_f32.to_bits());
    assert_eq!(p.y.to_bits(), 2.5_f32.to_bits());
    assert_eq!(p.z.to_bits(), 3.0_f32.to_bits());
}

#[test]
fn cell_center_sits_at_half_cell_offset_on_level_floor() {
    // The center of cell (4, 7) on level 2 is half a cell in on x/y and on
    // the level-2 floor (z = 2.0 exactly, the 0.0 level-fraction).
    let center = cell_center(Cell::new(4, 7), Level::new(2));
    assert_eq!(center.x.to_bits(), 4.5_f32.to_bits());
    assert_eq!(center.y.to_bits(), 7.5_f32.to_bits());
    assert_eq!(center.z.to_bits(), 2.0_f32.to_bits());
}

#[test]
fn cell_center_pos_to_cell_round_trip() {
    // cell_center → pos_to_cell is the identity on the (cell, level) it
    // names, for a positive cell.
    let cell = Cell::new(3, 9);
    let level = Level::new(4);
    let (back_cell, back_level) = pos_to_cell(cell_center(cell, level));
    assert_eq!(back_cell, cell);
    assert_eq!(back_level, level);
}

#[test]
fn pos_to_cell_floors_negative_coordinates() {
    // The floor-not-round contract: a NEGATIVE-coordinate cell must bucket
    // to the lower integer, never toward zero. cell_center(-2, -3) sits at
    // (-1.5, -2.5), which floors back to (-2, -3) — a round would give
    // (-1, -2) or (-2, -2) depending on the half, which is wrong.
    let cell = Cell::new(-2, -3);
    let level = Level::new(0);
    let (back_cell, back_level) = pos_to_cell(cell_center(cell, level));
    assert_eq!(back_cell, cell, "negative cell must floor, not round");
    assert_eq!(back_level, level);

    // A point anywhere inside a negative cell floors to that cell's corner.
    let (mid_cell, _) = pos_to_cell(SimPos::new(-0.1, -0.9, 0.0));
    assert_eq!(
        mid_cell,
        Cell::new(-1, -1),
        "a fractional negative coordinate floors to the lower cell",
    );
}

#[test]
fn pos_to_cell_levels_floor_within_a_storey() {
    // z within a storey floors to that storey: 2.0..3.0 → level 2.
    let (_, low_in_storey) = pos_to_cell(SimPos::new(0.5, 0.5, 2.0));
    let (_, high_in_storey) = pos_to_cell(SimPos::new(0.5, 0.5, 2.99));
    assert_eq!(low_in_storey, Level::new(2));
    assert_eq!(high_in_storey, Level::new(2));
}

/// GTW-205 AC2 — a `CellLevel` authored in RON as a `(cell, level)` pair
/// deserializes to a value bit-equal to the result of
/// `CellLevel::new(Cell::new(x, y), Level::new(l))` for the same authored
/// `x, y, l` — proving the storey-index `z` came through the typed constructor
/// (the AC2 invariant), not a raw continuous height. Equality to the
/// constructor result, not a pinned coordinate scheme.
#[test]
fn cell_level_deserializes_through_its_typed_constructor() {
    // Authored shape: a typed Cell pair + a bare storey Level.
    let authored = "(cell: (x: 7, y: 8), level: 3)";
    let parsed = ron::from_str::<CellLevel>(authored);
    assert!(parsed.is_ok(), "CellLevel RON must parse: {parsed:?}");
    let Ok(key) = parsed else {
        return;
    };
    // Equal to the value the typed constructor produces for the same coords —
    // the storey index flowed through Level → CellLevel::new, never a free z.
    assert_eq!(
        key,
        CellLevel::new(Cell::new(7, 8), Level::new(3)),
        "deserialized CellLevel must equal the typed-constructor result",
    );
    // And the inner key's z is exactly the constructed storey index (3), via
    // the public Deref accessors — never a continuous height.
    assert_eq!(key.x, 7);
    assert_eq!(key.y, 8);
    assert_eq!(key.z, 3);
}
