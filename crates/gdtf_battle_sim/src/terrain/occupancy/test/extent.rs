use super::super::{GRID_HEIGHT, GRID_WIDTH, GridExtent};
use crate::metric::CellUnit;

#[test]
fn grid_extent_round_trips_the_real_grid_extents() {
    assert_eq!(
        usize::try_from(*CellUnit::from(GridExtent::new(GRID_WIDTH))),
        Ok(GRID_WIDTH),
        "the shared conversion must carry GRID_WIDTH back unchanged",
    );
    assert_eq!(
        usize::try_from(*CellUnit::from(GridExtent::new(GRID_HEIGHT))),
        Ok(GRID_HEIGHT),
        "the shared conversion must carry GRID_HEIGHT back unchanged",
    );
}

#[test]
fn a_grid_extent_past_i32_saturates_at_its_max() {
    assert_eq!(
        CellUnit::from(GridExtent::new(usize::MAX)),
        CellUnit::new(i32::MAX),
        "an extent no i32 can hold must saturate at i32::MAX, never wrap or fall back to zero",
    );
}
