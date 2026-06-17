//! Unit tests for the world camera: render-layer isolation, framing geometry, the
//! pan-navigation pure helpers, the GTW-263 zoom, and the GTW-271 viewport-aware edge gate.

use bevy::{camera::visibility::RenderLayers, prelude::*};

use super::{
    framing::{camera_focus, clamp_camera},
    marker::{WORLD_RENDER_LAYER, WorldCamera, spawn_world_camera},
    pan::{
        EdgeBandPx, PanSpeed, StickDeadzone, keyboard_pan_dir, mouse_edge_dir, pan_camera,
        pan_velocity, stick_pan_dir, viewport_edge_dir,
    },
};

/// The configured world render layer does not intersect the UI camera's default
/// layer 0 (the local guarantee behind AC3) — exercised through the real
/// `RenderLayers::intersects` so it is genuine runtime behaviour, not a constant.
#[test]
fn world_render_layer_misses_layer_zero() {
    let world_layer = RenderLayers::layer(WORLD_RENDER_LAYER);
    let ui_layer = RenderLayers::layer(0);
    assert!(
        !world_layer.intersects(&ui_layer),
        "the world camera's render layer ({WORLD_RENDER_LAYER}) must not intersect the UI \
         camera's default layer 0",
    );
}

/// AC1 — `camera_focus` is the centroid (mean) of the supplied points, and `None`
/// for an empty iterator (so the framing leaves the camera be when no player gangers
/// exist). Pure, no `App`.
#[test]
fn camera_focus_is_the_centroid_or_none() {
    // Three points -> their mean.
    let three = [
        Vec2::new(0.0, 0.0),
        Vec2::new(6.0, 0.0),
        Vec2::new(0.0, 9.0),
    ];
    let mean = camera_focus(three);
    assert!(mean.is_some(), "three points must have a centroid");
    let Some(mean) = mean else { return };
    assert_eq!(
        mean.x.to_bits(),
        2.0_f32.to_bits(),
        "mean x = (0+6+0)/3 = 2"
    );
    assert_eq!(
        mean.y.to_bits(),
        3.0_f32.to_bits(),
        "mean y = (0+0+9)/3 = 3"
    );

    // One point -> that point.
    let one = camera_focus([Vec2::new(4.0, -7.0)]);
    assert!(one.is_some(), "one point must have a centroid");
    let Some(one) = one else { return };
    assert_eq!(
        one,
        Vec2::new(4.0, -7.0),
        "a single point is its own centroid"
    );

    // Empty -> None.
    assert!(
        camera_focus(std::iter::empty::<Vec2>()).is_none(),
        "an empty iterator has no centroid",
    );
}

/// AC2 — `clamp_camera` keeps the viewport inside the bounds and centres on the map
/// midpoint when the map is smaller than the viewport. Relations, not pinned scene
/// magnitudes. Pure, no `App`.
#[test]
fn clamp_camera_keeps_viewport_inside_and_centres_when_smaller() {
    let half = Vec2::new(10.0, 10.0);
    let world_min = Vec2::new(0.0, 0.0);
    let world_max = Vec2::new(100.0, 100.0);

    // Pushed past world_max -> clamped to world_max - half on that axis.
    let past_max = clamp_camera(Vec2::new(1000.0, 1000.0), half, world_min, world_max);
    assert_eq!(
        past_max,
        world_max - half,
        "a translation past world_max clamps to world_max - half",
    );

    // Pushed past world_min -> clamped to world_min + half on that axis.
    let past_min = clamp_camera(Vec2::new(-1000.0, -1000.0), half, world_min, world_max);
    assert_eq!(
        past_min,
        world_min + half,
        "a translation past world_min clamps to world_min + half",
    );

    // A within-bounds translation is unchanged.
    let inside = Vec2::new(50.0, 40.0);
    assert_eq!(
        clamp_camera(inside, half, world_min, world_max),
        inside,
        "a translation already inside the bounds is left unchanged",
    );

    // Map SMALLER than the viewport on both axes (2*half > span) -> the map midpoint.
    let big_half = Vec2::new(80.0, 80.0); // 2*80 = 160 > 100 span on each axis.
    let centred = clamp_camera(Vec2::new(1000.0, -1000.0), big_half, world_min, world_max);
    let midpoint = (world_min + world_max) * 0.5;
    assert_eq!(
        centred, midpoint,
        "when the map is narrower than the viewport, the camera centres on the midpoint",
    );
}

// -----------------------------------------------------------------------------
// GTW-250 — pan-navigation pure helpers (AC1–AC4).
// -----------------------------------------------------------------------------

/// A test window size for the mouse-edge helper.
const SIZE: Vec2 = Vec2::new(800.0, 600.0);
/// A test mouse-edge band.
const EDGE: EdgeBandPx = EdgeBandPx::new(20.0);

/// AC1 — `mouse_edge_dir` maps each edge band to a camera direction WITH the
/// screen-y → camera-y flip: top → `+Y`, bottom → `-Y`, left → `-X`, right → `+X`, a
/// corner → a diagonal, and the centre → `ZERO`. Relations, not pinned magnitudes.
#[test]
fn mouse_edge_dir_maps_each_edge_with_the_y_flip() {
    // TOP band (small screen y) pans the camera UP (+Y) — the explicit flip.
    let top = mouse_edge_dir(Vec2::new(SIZE.x * 0.5, 5.0), SIZE, EDGE);
    assert!(
        top.y > 0.0,
        "cursor near the TOP must pan the camera UP (+Y)"
    );
    assert_eq!(
        top.x.to_bits(),
        0.0_f32.to_bits(),
        "a centred-x top has no x pan"
    );

    // BOTTOM band (large screen y) pans the camera DOWN (-Y).
    let bottom = mouse_edge_dir(Vec2::new(SIZE.x * 0.5, SIZE.y - 5.0), SIZE, EDGE);
    assert!(
        bottom.y < 0.0,
        "cursor near the BOTTOM must pan the camera DOWN (-Y)"
    );

    // LEFT band → -X.
    let left = mouse_edge_dir(Vec2::new(5.0, SIZE.y * 0.5), SIZE, EDGE);
    assert!(left.x < 0.0, "cursor near the LEFT must pan -X");
    assert_eq!(
        left.y.to_bits(),
        0.0_f32.to_bits(),
        "a centred-y left has no y pan"
    );

    // RIGHT band → +X.
    let right = mouse_edge_dir(Vec2::new(SIZE.x - 5.0, SIZE.y * 0.5), SIZE, EDGE);
    assert!(right.x > 0.0, "cursor near the RIGHT must pan +X");

    // CORNER (top-right) → a diagonal: +X and +Y.
    let corner = mouse_edge_dir(Vec2::new(SIZE.x - 5.0, 5.0), SIZE, EDGE);
    assert!(
        corner.x > 0.0 && corner.y > 0.0,
        "the top-right corner must pan diagonally (+X, +Y)"
    );

    // CENTRE (no band) → ZERO.
    assert_eq!(
        mouse_edge_dir(SIZE * 0.5, SIZE, EDGE),
        Vec2::ZERO,
        "a cursor in the centre (no edge band) contributes no pan",
    );
}

/// AC2 — `keyboard_pan_dir` maps W/A/S/D to camera axes, opposite keys cancel, and
/// (via the system) arrows alias WASD. W → `+Y`, S → `-Y`, A → `-X`, D → `+X`,
/// W+D → up-right, W+S → `ZERO`.
#[test]
fn keyboard_pan_dir_combines_keys_and_cancels_opposites() {
    // Single keys (W / S / A / D).
    assert_eq!(
        keyboard_pan_dir(true, false, false, false),
        Vec2::new(0.0, 1.0),
        "W → +Y",
    );
    assert_eq!(
        keyboard_pan_dir(false, true, false, false),
        Vec2::new(0.0, -1.0),
        "S → -Y",
    );
    assert_eq!(
        keyboard_pan_dir(false, false, true, false),
        Vec2::new(-1.0, 0.0),
        "A → -X",
    );
    assert_eq!(
        keyboard_pan_dir(false, false, false, true),
        Vec2::new(1.0, 0.0),
        "D → +X",
    );

    // Combination: W+D → up-right.
    assert_eq!(
        keyboard_pan_dir(true, false, false, true),
        Vec2::new(1.0, 1.0),
        "W+D → up-right (+X, +Y)",
    );

    // Opposite keys cancel.
    assert_eq!(
        keyboard_pan_dir(true, true, false, false),
        Vec2::ZERO,
        "W+S cancel → ZERO",
    );
    assert_eq!(
        keyboard_pan_dir(false, false, true, true),
        Vec2::ZERO,
        "A+D cancel → ZERO",
    );
}

/// AC3 — `stick_pan_dir` zeroes a sub-deadzone stick and passes a clearly-past stick
/// through with the camera-y orientation preserved (stick up → +Y).
#[test]
fn stick_pan_dir_respects_the_deadzone() {
    let deadzone = StickDeadzone::new(0.15);

    // A tiny resting drift below the deadzone → ZERO.
    assert_eq!(
        stick_pan_dir(Vec2::new(0.05, -0.05), deadzone),
        Vec2::ZERO,
        "a sub-deadzone stick contributes no pan",
    );

    // A clear push past the deadzone passes through, magnitude preserved, stick-up = +Y.
    let pushed = Vec2::new(0.0, 0.8);
    assert_eq!(
        stick_pan_dir(pushed, deadzone),
        pushed,
        "a clearly-past stick passes through unchanged (stick up → camera +Y)",
    );
    assert!(
        stick_pan_dir(Vec2::new(0.0, 0.8), deadzone).y > 0.0,
        "stick UP must map to camera +Y",
    );
}

/// AC4 — `pan_velocity` scales a unit direction by speed, returns ZERO for no input,
/// and the diagonal-not-faster rule holds for the keyboard combination (a normalised
/// diagonal is no faster than a cardinal).
#[test]
fn pan_velocity_scales_and_diagonal_is_not_faster() {
    let speed = PanSpeed::new(100.0);

    // ZERO direction → ZERO velocity.
    assert_eq!(
        pan_velocity(Vec2::ZERO, speed),
        Vec2::ZERO,
        "no input → no velocity (no drift)",
    );

    // A unit cardinal → speed-scaled.
    let cardinal = pan_velocity(Vec2::new(0.0, 1.0), speed);
    assert_eq!(
        cardinal,
        Vec2::new(0.0, 100.0),
        "a unit direction scales to exactly `speed` world-units/sec",
    );

    // Diagonal-not-faster: the (normalised) diagonal speed equals the cardinal speed.
    let diagonal = pan_velocity(Vec2::new(1.0, 1.0), speed);
    let diag_speed = diagonal.length();
    let card_speed = cardinal.length();
    assert!(
        (diag_speed - card_speed).abs() < 1e-3,
        "a diagonal keyboard combo must not be faster than a cardinal (got diag {diag_speed}, \
         cardinal {card_speed})",
    );

    // A sub-unit analog magnitude scales speed DOWN (a gentle stick push pans gently).
    let gentle = pan_velocity(Vec2::new(0.0, 0.5), speed);
    assert!(
        gentle.length() < card_speed,
        "a sub-unit analog stick magnitude keeps its sub-unit scale (pans slower)",
    );
}

// -----------------------------------------------------------------------------
// GTW-263 — battle zoom: the world camera spawns at orthographic scale 0.5 (2x).
// -----------------------------------------------------------------------------

/// GTW-263 — `spawn_world_camera` spawns the `WorldCamera` with an orthographic
/// projection at `scale = 0.5` (a 2x zoom-in), OVERRIDING the `Camera2d` default of `1.0`.
///
/// Pin-discriminating: if the explicit projection were dropped, the `#[require]`d
/// `default_2d` projection (`scale = 1.0`) would be on the entity instead and the assert
/// would fail. Halving the scale halves the visible half-extent (`window * 0.5 * scale`),
/// so the battlefield draws at twice the size — the play-test "too zoomed out" fix.
#[test]
fn world_camera_spawns_at_half_orthographic_scale() {
    let mut app = App::new();
    app.add_systems(Startup, spawn_world_camera);
    app.update();

    let mut cameras = app
        .world_mut()
        .query_filtered::<&Projection, With<WorldCamera>>();
    let scale = cameras.iter(app.world()).next().and_then(|projection| {
        if let Projection::Orthographic(ortho) = projection {
            Some(ortho.scale)
        } else {
            None
        }
    });
    assert_eq!(
        scale.map(f32::to_bits),
        Some(0.5_f32.to_bits()),
        "the world camera must spawn one orthographic projection at scale 0.5 (2x zoom), \
         not the default_2d 1.0",
    );
}

// -----------------------------------------------------------------------------
// GTW-271 — edge-pan keys off the MAP VIEWPORT rect, not the window.
// -----------------------------------------------------------------------------

/// A test map-viewport rect: a 600x400 central region inset 100px from the left and 50px
/// from the top of an 800x600 window (the status panel on the left, a minimal top inset).
const VIEWPORT: Rect = Rect {
    min: Vec2::new(100.0, 50.0),
    max: Vec2::new(700.0, 450.0),
};

/// GTW-271 AC4 — `viewport_edge_dir` pans ONLY when the cursor is INSIDE the map viewport
/// rect AND within the edge band; a cursor OUTSIDE the rect (in a UI margin / over a panel)
/// → `Vec2::ZERO` (no pan). Inside-near-an-edge it reproduces the window-edge logic measured
/// from the VIEWPORT edges (with the screen-y → camera-y flip).
///
/// Pin-discriminating: it proves a cursor in the LEFT MARGIN (left of the viewport min, where
/// the status panel sits) does NOT pan even though it is near the WINDOW's left edge — the
/// behaviour the old window-relative `mouse_edge_dir` got wrong (it would have panned -X under
/// the panel). And a cursor just INSIDE the viewport's left edge DOES pan -X. Pure, no `App`.
#[test]
fn viewport_edge_dir_pans_only_inside_the_map_rect() {
    // A cursor in the LEFT MARGIN (x < viewport.min.x — over the status panel): no pan, even
    // though it is hard against the WINDOW's left edge.
    let in_margin = viewport_edge_dir(Vec2::new(10.0, 250.0), VIEWPORT, EDGE);
    assert_eq!(
        in_margin,
        Vec2::ZERO,
        "a cursor in the left margin (outside the map viewport) must NOT pan",
    );

    // A cursor BELOW the viewport (y > viewport.max.y — over the action-bar margin): no pan.
    let below = viewport_edge_dir(Vec2::new(400.0, 580.0), VIEWPORT, EDGE);
    assert_eq!(
        below,
        Vec2::ZERO,
        "a cursor below the map viewport (over the action-bar margin) must NOT pan",
    );

    // A cursor just INSIDE the viewport's LEFT edge (within the band of viewport.min.x) → -X.
    let near_left = viewport_edge_dir(Vec2::new(VIEWPORT.min.x + 5.0, 250.0), VIEWPORT, EDGE);
    assert!(
        near_left.x < 0.0,
        "a cursor just inside the viewport's left edge must pan -X (got {near_left:?})",
    );

    // A cursor just INSIDE the viewport's TOP edge → +Y (the screen-y → camera-y flip,
    // measured from the VIEWPORT top, not the window top).
    let near_top = viewport_edge_dir(Vec2::new(400.0, VIEWPORT.min.y + 5.0), VIEWPORT, EDGE);
    assert!(
        near_top.y > 0.0,
        "a cursor just inside the viewport's top edge must pan UP (+Y) (got {near_top:?})",
    );

    // A cursor in the CENTRE of the viewport (no edge band) → ZERO.
    let centre = viewport_edge_dir(VIEWPORT.min + VIEWPORT.size() * 0.5, VIEWPORT, EDGE);
    assert_eq!(
        centre,
        Vec2::ZERO,
        "a cursor in the centre of the map viewport contributes no pan",
    );
}

/// GTW-271 — `pan_camera`'s KEYBOARD source is NOT cursor-bound: it pans regardless of where
/// the cursor is (the GTW-262 `Interaction`-over-UI early-return that wrongly froze the
/// keyboard pan is GONE). This pins the refactor-preserved half of the pan system through the
/// real registered system, with the camera now carrying a `Camera` component (the AC4 query
/// reads `&Camera` for `logical_viewport_rect`).
///
/// Pin-discriminating: W is pressed and the keyboard pan must move the camera +Y across an
/// update — if the removed `pointer_over_ui` gate were still suppressing all sources, or if
/// the keyboard branch regressed, the camera would not move.
#[test]
fn pan_camera_keyboard_pans_regardless_of_cursor() {
    use std::time::Duration;

    use bevy::{
        time::TimeUpdateStrategy,
        window::{PrimaryWindow, Window},
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // A controlled non-zero per-update delta so the pan is measurable (the virtual clock
    // reports 0 on the first update and clamps each step to 250ms — ample at PAN_SPEED).
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::KeyW);
    app.insert_resource(keys);
    app.add_systems(Update, pan_camera);

    // The camera carries a `Camera` component (the AC4 query reads `&Camera`); headlessly
    // `logical_viewport_rect()` is `None`, so the mouse-edge source contributes nothing and
    // only the (cursor-independent) keyboard source drives the pan.
    app.world_mut().spawn((
        WorldCamera,
        Camera::default(),
        Transform::from_translation(Vec3::ZERO),
    ));
    app.world_mut().spawn((Window::default(), PrimaryWindow));

    // Warm up once (the first-update zero delta), then measure the keyboard pan.
    app.update();
    let before = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Transform, With<WorldCamera>>();
        q.iter(app.world()).next().map_or(0.0, |t| t.translation.y)
    };
    app.update();
    let after = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Transform, With<WorldCamera>>();
        q.iter(app.world()).next().map_or(0.0, |t| t.translation.y)
    };
    assert!(
        after > before,
        "with W pressed the keyboard pan must move the camera up (+Y) regardless of the \
         cursor: {before} -> {after}",
    );
}
