use super::super::*;
use crate::metric::MAX_LEVELS;

#[test]
fn grid_size_accepts_within_bounds() {
    let size = GridSize::new(
        GridWidth::new(MAX_GRID_SPAN),
        GridHeight::new(MAX_GRID_SPAN),
        GridLevels::new(MAX_LEVELS),
    );
    assert!(
        size.is_ok(),
        "a 60x60x8 grid is the legal maximum: {size:?}"
    );
    if let Ok(size) = size {
        assert_eq!(*size.width(), MAX_GRID_SPAN);
        assert_eq!(*size.height(), MAX_GRID_SPAN);
        assert_eq!(*size.levels(), MAX_LEVELS);
    }
}

#[test]
fn grid_size_rejects_over_max_per_axis() {
    let too_wide = GridSize::new(
        GridWidth::new(MAX_GRID_SPAN + 1),
        GridHeight::new(10),
        GridLevels::new(4),
    );
    assert!(
        matches!(too_wide, Err(GridSizeError::WidthOverMax(_))),
        "an over-max width is rejected naming the width axis, got {too_wide:?}",
    );

    let too_tall = GridSize::new(
        GridWidth::new(10),
        GridHeight::new(MAX_GRID_SPAN + 1),
        GridLevels::new(4),
    );
    assert!(
        matches!(too_tall, Err(GridSizeError::HeightOverMax(_))),
        "an over-max height is rejected naming the height axis, got {too_tall:?}",
    );

    let too_deep = GridSize::new(
        GridWidth::new(10),
        GridHeight::new(10),
        GridLevels::new(MAX_LEVELS + 1),
    );
    assert!(
        matches!(too_deep, Err(GridSizeError::LevelsOverMax(_))),
        "an over-max level count is rejected naming the levels axis, got {too_deep:?}",
    );
}

#[test]
fn grid_size_rejects_empty() {
    let empty = GridSize::new(GridWidth::new(0), GridHeight::new(10), GridLevels::new(4));
    assert!(
        matches!(empty, Err(GridSizeError::Empty { .. })),
        "a zero-span axis is rejected as Empty, got {empty:?}",
    );
}

#[test]
fn grid_size_deserializes_through_validation() {
    let ok = ron::de::from_str::<GridSize>("(width: 40, height: 30, levels: 4)");
    assert!(
        ok.is_ok(),
        "a within-bounds authored GridSize parses: {ok:?}"
    );

    let bad = ron::de::from_str::<GridSize>("(width: 99, height: 30, levels: 4)");
    assert!(
        bad.is_err(),
        "an over-max authored GridSize fails deserialization (validation runs through \
         try_from), got {bad:?}",
    );
}
