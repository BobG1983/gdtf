//! [`PREVIEW_VIEW_SPAN`]-world-unit view (independent of the offscreen texture's pixel size, so
use bevy::math::Vec2;
use gdtf_battle_sim::prelude::Cell;

pub(crate) const CELL_WORLD: f32 = 16.0;

pub(crate) const PREVIEW_VIEW_SPAN: f32 = 320.0;

#[must_use]
pub(crate) fn cell_center_world(cell: Cell) -> Vec2 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "cell coords are small grid indices (0..=60); the f32 cast is exact well within \
                  f32's 24-bit integer range"
    )]
    let (x, y) = (cell.x as f32, cell.y as f32);
    Vec2::new(x * CELL_WORLD, -y * CELL_WORLD)
}

/// visible world span is `PREVIEW_VIEW_SPAN * scale`, centred at `pan`. So a UV `(u, v)`:
#[must_use]
pub(crate) fn uv_to_cell(uv: Vec2, scale: f32, pan: Vec2) -> Cell {
    let span = PREVIEW_VIEW_SPAN * scale;
    let world_x = (uv.x - 0.5).mul_add(span, pan.x);
    let world_y = (-(uv.y - 0.5)).mul_add(span, pan.y);
    let cell_x = (world_x / CELL_WORLD).round();
    let cell_y = (-world_y / CELL_WORLD).round();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the rounded grid coordinate is a small integer; the caller clamps out-of-bounds \
                  cells via the EditorMap / evaluate_placement bounds check, so an out-of-range \
                  value is rejected downstream rather than silently wrapping into the grid"
    )]
    let cell = Cell::new(cell_x as i32, cell_y as i32);
    cell
}

#[must_use]
pub(crate) fn uv_to_world(uv: Vec2, scale: f32, pan: Vec2) -> Vec2 {
    let span = PREVIEW_VIEW_SPAN * scale;
    Vec2::new(
        (uv.x - 0.5).mul_add(span, pan.x),
        (-(uv.y - 0.5)).mul_add(span, pan.y),
    )
}

#[cfg(test)]
mod tests {
    use bevy::math::Vec2;
    use gdtf_battle_sim::prelude::Cell;

    use super::{CELL_WORLD, cell_center_world, uv_to_cell, uv_to_world};

            #[test]
    fn cell_center_flips_y_and_scales() {
        let w = cell_center_world(Cell::new(2, 3));
        let expected_x = 2.0 * CELL_WORLD;
        let expected_y = -3.0 * CELL_WORLD;
        assert!(
            (w.x - expected_x).abs() < f32::EPSILON,
            "x scales by CELL_WORLD"
        );
        assert!(
            (w.y - expected_y).abs() < f32::EPSILON,
            "grid +y renders DOWN-screen (negative world y)",
        );
    }

        #[test]
    fn center_uv_at_origin_maps_to_cell_origin() {
        let cell = uv_to_cell(Vec2::new(0.5, 0.5), 1.0, Vec2::ZERO);
        assert_eq!(
            cell,
            Cell::new(0, 0),
            "the image centre at origin pan is cell (0,0)"
        );
    }

            #[test]
    fn uv_round_trips_a_cell_center() {
        let scale = 1.0;
        let pan = Vec2::new(48.0, -32.0);
        for cell in [
            Cell::new(0, 0),
            Cell::new(3, 5),
            Cell::new(9, 1),
            Cell::new(1, 9),
        ] {
            let world = cell_center_world(cell);
            let span = super::PREVIEW_VIEW_SPAN * scale;
            let uv = Vec2::new(
                (world.x - pan.x) / span + 0.5,
                0.5 - (world.y - pan.y) / span,
            );
            let round_tripped = uv_to_cell(uv, scale, pan);
            assert_eq!(
                round_tripped, cell,
                "a UV on cell {cell:?}'s world centre must recover that cell (round-trip)",
            );
        }
    }

            #[test]
    fn uv_to_world_agrees_with_uv_to_cell() {
        let scale = 2.0;
        let pan = Vec2::new(-16.0, 8.0);
        let uv = Vec2::new(0.7, 0.35);
        let world = uv_to_world(uv, scale, pan);
        let via_world = Cell::new(
            (world.x / CELL_WORLD).round() as i32,
            (-world.y / CELL_WORLD).round() as i32,
        );
        assert_eq!(
            via_world,
            uv_to_cell(uv, scale, pan),
            "world→cell agrees with uv→cell"
        );
    }
}
