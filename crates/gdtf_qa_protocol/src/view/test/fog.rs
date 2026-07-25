//! The fixed-size fog handout pin (GTW-763).

use crate::view::{ExploredCellCountNet, FogView, VisibleCellCountNet};

/// [`FogView`] is a FIXED size regardless of how much the squad can see: it carries two
/// counts, never per-cell lists. Even a fully-visible `60×60×8` grid (`28_800` cells)
/// serializes to a tiny string. This FAILS the instant anyone reverts the counts to
/// `Vec<CellLevelNet>` (a list that long serializes to tens of thousands of chars).
/// See GTW-763.
#[test]
fn fog_view_is_constant_size_regardless_of_visibility() {
    /// The `60×60×8` grid's total cell count — the ceiling a fully-visible squad reaches.
    const FULL_GRID_CELLS: u32 = 60 * 60 * 8;

    let fog = FogView::new(
        VisibleCellCountNet::new(FULL_GRID_CELLS),
        ExploredCellCountNet::new(FULL_GRID_CELLS),
    );
    let Ok(encoded) = ron::ser::to_string(&fog) else {
        unreachable!("a fog view serializes to compact RON: {fog:?}");
    };
    assert!(
        encoded.len() < 200,
        "a fully-visible fog view must stay compact (counts, not cell lists); \
         got {} chars: {encoded}",
        encoded.len()
    );
}
