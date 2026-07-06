//! Bounds clamp: the hard pull-back + the relaxed margin band (AC4, GTW-381 AC5).

use bevy::{
    MinimalPlugins,
    app::{App, Update},
    camera::{OrthographicProjection, Projection},
    ecs::schedule::SystemCondition,
    math::Vec2,
    prelude::{Camera2d, IntoScheduleConfigs, Transform, resource_exists},
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_presenter::{WorldCamera, clamp_camera_to_bounds};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{BattleInProgress, Faction},
};

use super::harness::*;

/// The battlefield y-centre (the midpoint of the `(min, max)` ground extent on the y axis) — a
/// y a camera parks at so the relaxed clamp never touches the y axis while an x-edge case is tested.
fn field_centre_y() -> f32 {
    let (min, max) = battlefield_bounds();
    f32::midpoint(min.y, max.y)
}

// ---------------------------------------------------------------------------------
// AC4 — the clamp pulls an out-of-bounds camera back inside the battlefield.
// ---------------------------------------------------------------------------------

/// AC4 — a camera placed FAR outside the battlefield bounds, with a known orthographic
/// half-viewport + primary window, is pulled back inside `bounds + margin` by
/// `clamp_camera_to_bounds`.
///
/// GTW-381: with a KNOWN off-level margin inserted (`PanTuning`), the clamp relaxes the bounds
/// by that margin — the viewport must sit within the RELAXED `[min - margin, max + margin]`
/// box, NOT the bare hard bounds (which a margin > 0 grows past). This still exercises the
/// pull-back-an-out-of-bounds-camera mechanism on the real system.
#[test]
fn clamp_pulls_out_of_bounds_camera_back_inside() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_systems(
        Update,
        clamp_camera_to_bounds
            .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>)),
    );

    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(PLAYER_GANG)));
    // A KNOWN, non-zero off-level margin so the assertion is against the RELAXED bounds
    // deterministically (not the shipped default magnitude — that is a tunable, never pinned).
    app.world_mut().insert_resource(margin_tuning(MARGIN));

    // A primary window the clamp reads for the viewport-size fallback.
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(VIEWPORT.x as u32, VIEWPORT.y as u32),
            ..Default::default()
        },
        PrimaryWindow,
    ));

    // A WorldCamera with a computed orthographic `area` (the synthetic-camera recipe:
    // `projection.update(w, h)` sets `area` to the viewport-sized world rect, as bevy's
    // `camera_system` would). Place it FAR outside the battlefield (negative corner).
    let mut projection = Projection::Orthographic(OrthographicProjection::default_2d());
    projection.update(VIEWPORT.x, VIEWPORT.y);
    let far_outside = Vec2::new(-100_000.0, 100_000.0);
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        projection,
        Transform::from_xyz(far_outside.x, far_outside.y, 0.0),
    ));

    // Sanity: the camera starts far out of bounds.
    let before = camera_xy(&mut app);
    assert_eq!(
        before, far_outside,
        "precondition: the camera starts far out of bounds"
    );

    app.update();

    // The clamp must pull the camera inside the battlefield's world bounds. The bounds are
    // the 60x60 ground extent projected through `cell_to_world`; assert the relation (the
    // viewport stays within the bounds) rather than pinning a magnitude.
    let after = camera_xy(&mut app);
    assert_ne!(
        after, far_outside,
        "the clamp must MOVE a far-out-of-bounds camera",
    );

    // The battlefield bounds, RELAXED by the known margin, recomputed the same way the system
    // does, with the same half-viewport (area.half_size() == VIEWPORT/2 after `update`).
    let (hard_min, hard_max) = battlefield_bounds();
    let relaxed_min = hard_min - Vec2::splat(MARGIN);
    let relaxed_max = hard_max + Vec2::splat(MARGIN);
    let half = VIEWPORT * 0.5;
    // The viewport [after - half, after + half] must sit within the RELAXED [min, max] on each
    // axis (the battlefield is far larger than this small viewport, so the clamp is the boundary
    // case, not the centre-when-smaller case).
    assert!(
        after.x - half.x >= relaxed_min.x - f32::EPSILON
            && after.x + half.x <= relaxed_max.x + f32::EPSILON,
        "after the clamp the viewport must stay within the RELAXED (bounds + margin) x bounds",
    );
    assert!(
        after.y - half.y >= relaxed_min.y - f32::EPSILON
            && after.y + half.y <= relaxed_max.y + f32::EPSILON,
        "after the clamp the viewport must stay within the RELAXED (bounds + margin) y bounds",
    );
}

// ---------------------------------------------------------------------------------
// GTW-381 AC5 — the relaxed clamp: beyond bounds+margin is pulled back to that edge;
// within the margin is NOT clamped (stays where it panned).
// ---------------------------------------------------------------------------------

/// Spawns the AC5 fixture: a `WorldCamera` with a KNOWN orthographic half-viewport
/// (`area.half_size() == VIEWPORT/2` after `projection.update`) at `at`, a primary window, the
/// battle gate, and a `PanTuning` carrying [`MARGIN`]. Returns the app ready for one `update()`.
fn relaxed_clamp_app(at: Vec2) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_systems(
        Update,
        clamp_camera_to_bounds
            .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>)),
    );
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(PLAYER_GANG)));
    app.world_mut().insert_resource(margin_tuning(MARGIN));
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(VIEWPORT.x as u32, VIEWPORT.y as u32),
            ..Default::default()
        },
        PrimaryWindow,
    ));
    let mut projection = Projection::Orthographic(OrthographicProjection::default_2d());
    projection.update(VIEWPORT.x, VIEWPORT.y);
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        projection,
        Transform::from_xyz(at.x, at.y, 0.0),
    ));
    app
}

/// AC5 (pull-back) — a camera panned BEYOND `bounds + margin` is pulled back to EXACTLY the
/// `bounds + margin` edge by the real `clamp_camera_to_bounds`: not further, and (since the
/// margin is non-zero) NOT the old hard-bounds edge.
///
/// Drives the production system. The half-viewport is the known `VIEWPORT/2`, so the relaxed
/// clamp's x-max edge is `(hard_max + margin) - half_x` (the `clamp_camera` per-axis relation):
/// a camera placed far to the +x of that edge must land EXACTLY on it. Asserting the relation
/// (the relaxed edge), not a pinned scene magnitude — the margin is a tunable.
#[test]
fn relaxed_clamp_pulls_back_to_bounds_plus_margin() {
    let (_hard_min, hard_max) = battlefield_bounds();
    let half = VIEWPORT * 0.5;
    // The relaxed clamp edge on the +x axis: bounds + margin, inset by the half-viewport.
    let relaxed_edge_x = (hard_max.x + MARGIN) - half.x;
    let hard_edge_x = hard_max.x - half.x;
    // Start far PAST the relaxed edge (definitely out of bounds on +x), parked mid-field on y.
    let start = Vec2::new(relaxed_edge_x + 10_000.0, field_centre_y());

    let mut app = relaxed_clamp_app(start);
    app.update();
    let after = camera_xy(&mut app);

    // Pulled back to EXACTLY the relaxed (bounds + margin) edge on x.
    assert!(
        (after.x - relaxed_edge_x).abs() < f32::EPSILON,
        "a camera beyond bounds+margin must be pulled back to the bounds+margin edge ({relaxed_edge_x}), got {}",
        after.x,
    );
    // And NOT to the old hard-bounds edge — the relaxation is real (margin > 0 moved the edge out).
    assert!(
        (after.x - hard_edge_x).abs() > HALF_MARGIN,
        "the pull-back edge must be the RELAXED bounds+margin, NOT the old hard bounds ({hard_edge_x})",
    );
}

/// AC5 (within-margin) — a camera panned PAST the old hard bounds but still WITHIN the margin is
/// NOT clamped: it stays exactly where it was panned. The same position WOULD be pulled in under
/// the old hard clamp (it sits outside `[hard_min + half, hard_max - half]`), so the un-clamp is
/// real and caused by the margin — driven through the production `clamp_camera_to_bounds`.
#[test]
fn within_margin_camera_is_not_clamped() {
    let (_hard_min, hard_max) = battlefield_bounds();
    let half = VIEWPORT * 0.5;
    let hard_edge_x = hard_max.x - half.x;
    // Park HALF a margin past the old hard edge on +x: past the hard clamp, but inside the relaxed
    // clamp (`< hard_edge_x + margin`). Mid-field on y so y never clamps.
    let within = Vec2::new(hard_edge_x + HALF_MARGIN, field_centre_y());

    // Sanity (fixture invariant): `within` is OUTSIDE the old hard clamp range, so the un-clamp
    // below is genuinely due to the margin (the old hard clamp WOULD have pulled it in).
    assert!(
        within.x > hard_edge_x,
        "fixture invariant: the within-margin position must sit PAST the old hard-bounds edge",
    );

    let mut app = relaxed_clamp_app(within);
    app.update();
    let after = camera_xy(&mut app);

    // The relaxed clamp leaves it put — it is within `bounds + margin`.
    assert!(
        (after.x - within.x).abs() < f32::EPSILON && (after.y - within.y).abs() < f32::EPSILON,
        "a camera within bounds+margin must NOT be clamped — it stays where it panned ({within}), got {after}",
    );
}
