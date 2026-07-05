//! Shared `camera_frame` fixture: the camera consts, the known-margin tuning, the
//! battlefield bounds, and the camera-translation probe.

use bevy::{
    app::App,
    math::Vec2,
    prelude::{Transform, With},
};
use gdtf_battle_presenter::{BoundsMarginWorld, PanTuning, WorldCamera, cell_to_world};
use gdtf_battle_sim::{Cell, Level};

/// The player gang for the AC3 fixture.
pub(crate) const PLAYER_GANG: u8 = 0;
/// An enemy gang the framing must IGNORE (only the player's gangers frame the camera).
pub(crate) const ENEMY_GANG: u8 = 1;

/// A synthetic viewport size (physical px) for the AC4 clamp test.
pub(crate) const VIEWPORT: Vec2 = Vec2::new(640.0, 480.0);

/// A KNOWN, non-zero off-level pan margin (world units, GTW-381) the clamp tests insert via
/// [`margin_tuning`], so the assertions are against the RELAXED `bounds + margin` box
/// deterministically — NOT the shipped `BoundsMarginWorld::DEFAULT` magnitude (a tunable, never
/// pinned). Distinct from any plausible default so the relaxation is observable.
pub(crate) const MARGIN: f32 = 96.0;

/// Half the test [`MARGIN`] — the +x offset that parks a camera PAST the old hard edge but still
/// WITHIN the relaxed `bounds + margin` (a const so the FLOP is a compile-time value).
pub(crate) const HALF_MARGIN: f32 = MARGIN * 0.5;

/// Builds a [`PanTuning`] carrying a known off-level [`BoundsMarginWorld`], leaving every other
/// pan value at its shipped default — so the clamp tests drive the relaxed bounds on a KNOWN
/// margin rather than the shipped default.
pub(crate) fn margin_tuning(margin: f32) -> PanTuning {
    PanTuning {
        bounds_margin_world: BoundsMarginWorld::new(margin),
        ..PanTuning::default()
    }
}

/// Reads the single `WorldCamera`'s translation `xy`.
pub(crate) fn camera_xy(app: &mut App) -> Vec2 {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Transform, With<WorldCamera>>();
    let mut found = Vec2::ZERO;
    for transform in query.iter(world) {
        found = Vec2::new(transform.translation.x, transform.translation.y);
    }
    found
}

/// The battlefield ground-plane world bounds `(min, max)`, projected the SAME way the
/// system does (the four corner cells of the 60x60 ground extent through `cell_to_world`).
/// Kept here in the test so AC4 asserts a RELATION, not a pinned magnitude.
pub(crate) fn battlefield_bounds() -> (Vec2, Vec2) {
    let w = i32::try_from(gdtf_battle_sim::GRID_WIDTH).unwrap_or(i32::MAX);
    let h = i32::try_from(gdtf_battle_sim::GRID_HEIGHT).unwrap_or(i32::MAX);
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
