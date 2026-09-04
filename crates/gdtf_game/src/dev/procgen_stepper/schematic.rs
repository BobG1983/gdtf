use bevy_egui::egui;
use gdtf_battle_sim::{
    level::GridSize,
    procgen::{PlacedFootprint, PlacementRole, RegionRect},
};

/// `THUMB_EDGE` / `PREVIEW_MAX_EDGE` precedent: an egui screen size, not a domain quantity).
const SCHEMATIC_EDGE: f32 = 220.0;

const GRID_LINE_SPACING_CELLS: i32 = 10;

const GRID_STROKE: egui::Color32 = egui::Color32::from_gray(90);

fn schematic_scale(grid: GridSize, box_rect: egui::Rect) -> f32 {
    let span = (*grid.width()).max(*grid.height()).max(1);
    box_rect.width() / f32::from(span)
}

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
    let y_bottom = (origin.y as f32).mul_add(-scale, box_rect.max.y);
    let y_top = ((origin.y + fp.height()) as f32).mul_add(-scale, box_rect.max.y);
    egui::Rect::from_min_max(egui::pos2(x0, y_top), egui::pos2(x1, y_bottom))
}

const fn color_for(role: PlacementRole) -> egui::Color32 {
    match role {
        PlacementRole::Player => egui::Color32::from_rgb(70, 130, 220),
        PlacementRole::Enemy => egui::Color32::from_rgb(210, 80, 70),
        PlacementRole::Fill => egui::Color32::from_rgb(140, 140, 150),
    }
}

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
    /// Paint the placed footprints onto a fixed-size top-down schematic.
    fn draw_schematic(ui: &mut egui::Ui, grid: GridSize, footprints: &[PlacedFootprint]) {
        let (box_rect, _response) =
            ui.allocate_exact_size(egui::vec2(SCHEMATIC_EDGE, SCHEMATIC_EDGE), egui::Sense::hover());
        let scale = schematic_scale(grid, box_rect);
        let painter = ui.painter();
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

    #[test]
    fn footprint_rect_maps_a_region_with_a_y_flip() {
        let Ok(grid) = GridSize::new(GridWidth::new(20), GridHeight::new(20), GridLevels::new(1))
        else {
            return;
        };
        let box_rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(200.0, 200.0));
        let region = RegionRect::new(Cell::new(2, 3), Footprint::new(4, 5));
        let rect = footprint_rect(region, grid, box_rect);
        assert_eq!(
            rect,
            egui::Rect::from_min_max(egui::pos2(20.0, 120.0), egui::pos2(60.0, 170.0)),
        );
    }

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
