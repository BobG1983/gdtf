//! Unit tests for the hover-highlight draw layer.

use gdtf_battle_sim::{Cell, CellLevel, Level};

use super::draw::level_index;

/// `level_index` narrows a [`CellLevel`]'s storey `z` back to the `u8` the
/// [`Level`] carries, so the highlight redraws on the requested level.
#[test]
fn level_index_recovers_the_storey() {
    let key = CellLevel::new(Cell::new(2, 2), Level::new(3));
    assert_eq!(level_index(key), 3, "level_index must recover the storey z");
}
