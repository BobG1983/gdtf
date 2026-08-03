use bevy::{
    app::App,
    math::Vec2,
    prelude::{Transform, With},
};
use gdtf_battle_presenter::{BoundsMarginWorld, PanTuning, WorldCamera, cell_to_world};
use gdtf_battle_sim::prelude::{Cell, Level};

pub(crate) const PLAYER_GANG: u8 = 0;
pub(crate) const ENEMY_GANG: u8 = 1;

pub(crate) const VIEWPORT: Vec2 = Vec2::new(640.0, 480.0);

pub(crate) const MARGIN: f32 = 96.0;

pub(crate) const HALF_MARGIN: f32 = MARGIN * 0.5;

pub(crate) fn margin_tuning(margin: f32) -> PanTuning {
    PanTuning {
        bounds_margin_world: BoundsMarginWorld::new(margin),
        ..PanTuning::default()
    }
}

pub(crate) fn camera_xy(app: &mut App) -> Vec2 {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Transform, With<WorldCamera>>();
    let mut found = Vec2::ZERO;
    for transform in query.iter(world) {
        found = Vec2::new(transform.translation.x, transform.translation.y);
    }
    found
}

pub(crate) fn battlefield_bounds() -> (Vec2, Vec2) {
    let w = i32::try_from(gdtf_battle_sim::occupancy::GRID_WIDTH).unwrap_or(i32::MAX);
    let h = i32::try_from(gdtf_battle_sim::occupancy::GRID_HEIGHT).unwrap_or(i32::MAX);
    let corners = [
        cell_to_world(Cell::new(0, 0), Level::new(0)),
        cell_to_world(Cell::new(w, 0), Level::new(0)),
        cell_to_world(Cell::new(0, h), Level::new(0)),
        cell_to_world(Cell::new(w, h), Level::new(0)),
    ];
    let mut min = Vec2::new(corners[0].x, corners[0].y);
    let mut max = min;
    for c in corners {
        let p = Vec2::new(c.x, c.y);
        min = min.min(p);
        max = max.max(p);
    }
    (min, max)
}
