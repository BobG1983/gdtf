//! The stepper's **schematic overview** (GTW-732) — a pure egui-painter map of the procgen
//! cursor's placed footprints: a vague grid outline scaled to a fixed panel box, plus ONE
//! footprint shape appearing per placement as it lands.
//!
//! This is EXPLICITLY NOT the real terrain renderer — it reads the sim's render-free
//! [`PlacedFootprint`] list (role + placed [`RegionRect`]) and paints each as a coloured box
//! on a scaled grid, so a developer watches the level assemble piece by piece. The pure
//! geometry ([`footprint_rect`]) is unit-testable headlessly (`egui::Rect` is plain data); the
//! draw itself ([`draw_schematic`]) only READS the footprints and paints — no world/resource
//! mutation — so it is idempotent under a `bevy_egui` multipass re-run (bevy-traps #8).

use bevy_egui::egui;
use gdtf_battle_sim::{
    level::GridSize,
    procgen::{PlacedFootprint, PlacementRole, RegionRect},
};

/// The fixed square edge of the schematic box, in egui points (a framework-layout const — the
/// `THUMB_EDGE` / `PREVIEW_MAX_EDGE` precedent: an egui screen size, not a domain quantity).
const SCHEMATIC_EDGE: f32 = 220.0;

/// How many cells apart the faint gridlines are drawn — a cosmetic reference lattice so the
/// scaled grid reads as a grid (a layout const, not a placement quantity).
const GRID_LINE_SPACING_CELLS: i32 = 10;

/// The vague grid outline + gridline stroke colour (a faint grey).
const GRID_STROKE: egui::Color32 = egui::Color32::from_gray(90);

/// The cell-to-screen SCALE for the schematic box — the box edge over the larger grid span, so
/// the whole board fits the square box with uniform scaling (a framework-layout `f32`).
fn schematic_scale(grid: GridSize, box_rect: egui::Rect) -> f32 {
    let span = (*grid.width()).max(*grid.height()).max(1);
    box_rect.width() / f32::from(span)
}

/// Map a board [`RegionRect`] to its screen rectangle inside `box_rect`, with a y-FLIP: the
/// board's bottom-left ([`Anchor::BottomLeft`](gdtf_battle_sim::procgen::Anchor)) maps to the
/// box's bottom-left, so a placement's on-screen position matches its board position
/// (screen y grows down, board y grows up).
#[expect(
    clippy::cast_precision_loss,
    reason = "cell coordinates are tiny (<= 60); the f32 casts are exact within f32's 24-bit \
              integer range (the sprite_thumb sheet_uv precedent)"
)]
pub(crate) fn footprint_rect(
    region: RegionRect,
    grid: GridSize,
    box_rect: egui::Rect,
) -> egui::Rect {
    let scale = schematic_scale(grid, box_rect);
    let origin = region.origin();
    let fp = region.footprint();
    let x0 = (origin.x as f32).mul_add(scale, box_rect.min.x);
    let x1 = ((origin.x + fp.width()) as f32).mul_add(scale, box_rect.min.x);
    // y-flip: board cell y maps upward from the box bottom.
    let y_bottom = (origin.y as f32).mul_add(-scale, box_rect.max.y);
    let y_top = ((origin.y + fp.height()) as f32).mul_add(-scale, box_rect.max.y);
    egui::Rect::from_min_max(egui::pos2(x0, y_top), egui::pos2(x1, y_bottom))
}

/// The distinct fill colour for each placement role — player, enemy, and connective fill each
/// read at a glance.
const fn color_for(role: PlacementRole) -> egui::Color32 {
    match role {
        PlacementRole::Player => egui::Color32::from_rgb(70, 130, 220),
        PlacementRole::Enemy => egui::Color32::from_rgb(210, 80, 70),
        PlacementRole::Fill => egui::Color32::from_rgb(140, 140, 150),
    }
}

/// Draw the faint gridline lattice inside `box_rect` — a vertical + horizontal line every
/// [`GRID_LINE_SPACING_CELLS`] cells — so the scaled grid reads as a grid.
fn draw_gridlines(painter: &egui::Painter, grid: GridSize, box_rect: egui::Rect, scale: f32) {
    let stroke = egui::Stroke::new(0.5, GRID_STROKE);
    let width = i32::from(*grid.width());
    let height = i32::from(*grid.height());
    let mut cx = GRID_LINE_SPACING_CELLS;
    while cx < width {
        let x = (cx as f32).mul_add(scale, box_rect.min.x);
        painter.line_segment(
            [egui::pos2(x, box_rect.min.y), egui::pos2(x, box_rect.max.y)],
            stroke,
        );
        cx += GRID_LINE_SPACING_CELLS;
    }
    let mut cy = GRID_LINE_SPACING_CELLS;
    while cy < height {
        let y = (cy as f32).mul_add(-scale, box_rect.max.y);
        painter.line_segment(
            [egui::pos2(box_rect.min.x, y), egui::pos2(box_rect.max.x, y)],
            stroke,
        );
        cy += GRID_LINE_SPACING_CELLS;
    }
}

crate::support_item! {
    /// Draw the schematic overview into `ui`: allocate a fixed square box, paint the vague grid
    /// outline + gridlines, then one coloured box per placed footprint (GTW-732).
    ///
    /// Multipass-idempotent (bevy-traps #8): it only READS `grid` + `footprints` and paints —
    /// no world/resource/camera mutation — so a multipass re-run paints the identical picture.
    /// `pub` under `test-support` (so the re-export keeps it a reachable public API item, and
    /// the geometry stays exercised even when `super::ui` is excluded from headless builds),
    /// `pub(crate)` otherwise (`super::ui`'s draw system calls it).
    fn draw_schematic(ui: &mut egui::Ui, grid: GridSize, footprints: &[PlacedFootprint]) {
        let (box_rect, _response) =
            ui.allocate_exact_size(egui::vec2(SCHEMATIC_EDGE, SCHEMATIC_EDGE), egui::Sense::hover());
        let scale = schematic_scale(grid, box_rect);
        let painter = ui.painter();
        // The vague grid outline (four edges — a precedent-safe stroke via line segments).
        let outline = egui::Stroke::new(1.0, GRID_STROKE);
        let corners = [
            box_rect.left_top(),
            box_rect.right_top(),
            box_rect.right_bottom(),
            box_rect.left_bottom(),
        ];
        for i in 0..corners.len() {
            let a = corners[i];
            let b = corners[(i + 1) % corners.len()];
            painter.line_segment([a, b], outline);
        }
        draw_gridlines(painter, grid, box_rect, scale);
        // One footprint box per placement — appears the frame after its step lands it.
        for footprint in footprints {
            painter.rect_filled(
                footprint_rect(footprint.region(), grid, box_rect),
                0.0,
                color_for(footprint.role()),
            );
        }
    }
}

#[cfg(test)]
mod test {
    use bevy_egui::egui;
    use gdtf_battle_sim::{
        level::{GridHeight, GridLevels, GridSize, GridWidth},
        metric::Cell,
        procgen::{Footprint, RegionRect},
    };

    use super::footprint_rect;

    /// A 20x20 grid + a 200x200 box → scale 10; a region at origin `(2, 3)` footprint `4x5`
    /// maps to the y-FLIPPED screen rect: board bottom-left is the box bottom, so board y grows
    /// UP the screen (its top edge, `y = 3 + 5 = 8`, is at the SMALLER screen y).
    #[test]
    fn footprint_rect_maps_a_region_with_a_y_flip() {
        let Ok(grid) = GridSize::new(GridWidth::new(20), GridHeight::new(20), GridLevels::new(1))
        else {
            return;
        };
        let box_rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(200.0, 200.0));
        let region = RegionRect::new(Cell::new(2, 3), Footprint::new(4, 5));
        let rect = footprint_rect(region, grid, box_rect);
        // x: [2*10, 6*10]; y (flipped): top = 200 - 8*10 = 120, bottom = 200 - 3*10 = 170.
        assert_eq!(
            rect,
            egui::Rect::from_min_max(egui::pos2(20.0, 120.0), egui::pos2(60.0, 170.0)),
        );
    }

    /// A placement flush against the board's bottom-left (origin `(0, 0)`) sits flush against
    /// the box's bottom edge — the anchor of the y-flip.
    #[test]
    fn bottom_left_placement_sits_at_the_box_bottom() {
        let Ok(grid) = GridSize::new(GridWidth::new(20), GridHeight::new(20), GridLevels::new(1))
        else {
            return;
        };
        let box_rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(200.0, 200.0));
        let region = RegionRect::new(Cell::new(0, 0), Footprint::new(2, 2));
        let rect = footprint_rect(region, grid, box_rect);
        assert_eq!(
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 180.0), egui::pos2(20.0, 200.0)),
        );
    }
}
