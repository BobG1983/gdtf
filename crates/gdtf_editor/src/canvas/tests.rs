//! In-crate unit tests for the map-editor canvas (moved from the monolithic `canvas.rs`).

use bevy::ui::Val;
use gdtf_battle_sim::level::{GridHeight, GridWidth};

use super::types::{BOUNDARY_PX, CANVAS_CELL_PX, CELL_DASH_PX, CanvasExtent, grid_container_node};

/// T6 (GTW-463 C2): layout-guard — [`grid_container_node`]'s row-width must pack EXACTLY
/// `width` cells per row for sizes that expose the old `BorderBox` box-model bug.
///
/// **What this tests:** `grid_container_node` sets the container's `Node.width` to
/// `Val::Px(2*BOUNDARY_PX + cell_outer * width)`. The container's inner content width is
/// therefore `cell_outer * width`. Under Bevy's default `BorderBox` each cell's flex
/// footprint equals `CANVAS_CELL_PX` (24 px) — the 1 px dash border paints INSIDE the
/// 24 px — so the footprint is NOT `CANVAS_CELL_PX + 2*CELL_DASH_PX = 26 px`.
///
/// **Pre-fix failure (the bug):** the old formula used `cell_outer = 26 px`, so the
/// container outer width was `4 + 26*W` px and inner was `26*W`. Taffy flex-wraps at
/// `floor(26*W / 24)` cells per row: at W = 16 that is `17 ≠ 16` (right-overflow +
/// ragged bottom-right). At W = 8 it was `8` — a lucky fit that masked the bug in QA.
///
/// **Post-fix correctness:** `cell_outer = CANVAS_CELL_PX = 24 px`, so the container
/// outer width is `4 + 24*W` px and inner is `24*W`. `floor(24*W / 24) = W` exactly.
///
/// **Why this test catches a revert:** it calls the REAL `grid_container_node` and
/// extracts the `Val::Px` width it sets on the container. Reverting the fix (restoring
/// `cell_outer = 26 px`) changes the `Val::Px` value and breaks the `assert_eq!`.
/// The `assert_ne!` regression guard also confirms the BUG formula differs from `width`
/// at these widths — so if someone "fixes" the bug differently (e.g. with a different
/// constant), both sides of the check remain meaningful.
#[test]
fn grid_row_packs_exactly_width_cells_per_row() {
    // The cell's real flex footprint under BorderBox:
    // dash border (CELL_DASH_PX) is INSIDE the 24 px cell — footprint = CANVAS_CELL_PX.
    let cell_footprint = CANVAS_CELL_PX;

    // The OLD (buggy) `cell_outer`: CANVAS_CELL_PX + 2*CELL_DASH_PX = 26 px.
    let buggy_cell_outer = 2.0f32.mul_add(CELL_DASH_PX, CANVAS_CELL_PX);

    // For each bug-exposing width, call the REAL grid_container_node, extract its px width,
    // derive the inner content width, and assert it packs exactly `width` cells per row.
    for width in [16_u8, 20, 30, 60] {
        let extent = CanvasExtent {
            width:  GridWidth::new(width),
            height: GridHeight::new(8), // height does not affect the row-width formula
        };
        let node = grid_container_node(extent);

        // Extract the pixel width the node declares; it must be Val::Px for a
        // fixed-scale pixel grid. Use a sentinel so the assertion below can still
        // fire a meaningful message if the variant changes unexpectedly.
        let outer_px = if let Val::Px(px) = node.width {
            px
        } else {
            // Signal a non-Px width via a sentinel: cells_per_row below will be 0 and
            // the assert_eq will fail with a message identifying width + the bad variant.
            f32::NAN
        };
        assert!(
            outer_px.is_finite(),
            "grid_container_node must set Val::Px width, got {:?} at width {width} \
             (GTW-463 C2 guard)",
            node.width,
        );

        // Inner content width = outer - 2*BOUNDARY_PX (the container's own border).
        let inner_px = 2.0f32.mul_add(-BOUNDARY_PX, outer_px);

        // Cells per row = floor(inner_content_width / cell_footprint).
        // This is the formula taffy uses for flex-wrap row breaks.
        let cells_per_row = (inner_px / cell_footprint).floor() as u8;
        assert_eq!(
            cells_per_row, width,
            "grid_container_node at width {width}: inner width {inner_px}px must pack \
             exactly {width} cells per row at footprint {cell_footprint}px (GTW-463 C2)",
        );

        // Regression guard: the BUG formula (26*W) must pack ≠ width cells at these
        // widths — confirming the bug existed and the fix actually changes the outcome.
        // (Width 8 is excluded from the loop: floor(26*8/24)=8 was a lucky fit.)
        let buggy_inner = buggy_cell_outer * f32::from(width);
        let buggy_cells_per_row = (buggy_inner / cell_footprint).floor() as u8;
        assert_ne!(
            buggy_cells_per_row, width,
            "pre-fix regression guard: the old formula's inner width {buggy_inner}px should \
             pack {buggy_cells_per_row} cells per row at width {width}, NOT {width} — if \
             this fires, adjust the test width list (width is a lucky fit for the old formula)",
        );
    }
}
