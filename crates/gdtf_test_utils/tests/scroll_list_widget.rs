//! GTW-412 headless integration test for the [`ScrollList`](gdtf_ui::ScrollList) widget,
//! driving the REAL widget on the REAL `bevy_ui` layout path + the REAL built-in
//! [`ScrollAreaPlugin`](bevy::ui_widgets::ScrollAreaPlugin) scroll observer (verification
//! rule 3).
//!
//! These run on the [`GdtfUiTestAppBuilder`] `DefaultPlugins` headless harness (real layout
//! geometry — a computed [`ComputedNode`] — and a real camera) with [`gdtf_ui::UiPlugin`]
//! added, so the scroll-area's overflow + the scrollbar thumb actually compute. Each test
//! is pin-discriminating + value-agnostic: it would FAIL if the `overflow: scroll_y` /
//! `ScrollArea` / `ScrollAreaPlugin` wiring were reverted, and PASS only when a real wheel
//! scroll moves (and clamps) the production [`ScrollPosition`](bevy::ui::ScrollPosition).

use bevy::{
    camera::NormalizedRenderTarget,
    input::{mouse::MouseScrollUnit, touch::TouchPhase},
    picking::{
        backend::HitData,
        events::{Pointer, Scroll},
        pointer::{Location, PointerId},
    },
    prelude::*,
    ui::{ComputedNode, Node, PositionType, ScrollPosition, Val},
    ui_widgets::ScrollbarThumb,
    window::WindowRef,
};
use gdtf_test_utils::GdtfUiTestAppBuilder;
use gdtf_ui::{ScrollListColors, UiPlugin, spawn_scroll_list};

/// A distinct test color set so the build is exercised with real (non-default) colors.
const COLORS: ScrollListColors = ScrollListColors {
    area:  Color::srgb(0.10, 0.10, 0.14),
    track: Color::srgb(0.18, 0.18, 0.22),
    thumb: Color::srgb(0.55, 0.55, 0.62),
};

/// How many rows to stack — far more than fit in the short container, so the content
/// overflows its viewport on the Y axis.
const ROW_COUNT: usize = 20;

/// Builds the harness: the headless UI app + `UiPlugin` (which ensures the engine scroll
/// widgets' plugins are present), so the real `ScrollArea` observer + scrollbar thumb run.
fn harness() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(UiPlugin);
    app
}

/// The short container's height, in test-fixture pixels (the `Vh`/`Vw` relative units the
/// PRODUCTION widget uses resolve to ZERO under this windowless headless harness, where the
/// camera render target has no size — so the TEST fixture pins an explicit pixel viewport to
/// give the layout something concrete to overflow. This Px is in the test harness, NOT the
/// widget, so it is not a C2 relative-sizing violation).
const CONTAINER_HEIGHT_PX: f32 = 100.0;

/// Each row's fixed height, in test-fixture pixels. `ROW_COUNT` × this far exceeds
/// [`CONTAINER_HEIGHT_PX`], so the content stack overflows the viewport. (Same test-fixture
/// Px rationale as [`CONTAINER_HEIGHT_PX`].)
const ROW_HEIGHT_PX: f32 = 30.0;

/// Spawns a SHORT container holding a scroll list, parents [`ROW_COUNT`] tall rows onto the
/// scroll area, settles the layout, and returns the scroll-area entity.
///
/// The container is [`CONTAINER_HEIGHT_PX`] tall and each row is [`ROW_HEIGHT_PX`], so the
/// stacked content (`ROW_COUNT` × row) far exceeds the viewport — guaranteeing Y overflow.
/// `spawn_scroll_list` returns the AREA and parents it under its own root frame; this
/// re-parents that root under the short container so the frame fills it.
fn spawn_short_list_with_tall_rows(app: &mut App) -> Entity {
    let (container, area) = {
        let mut commands = app.world_mut().commands();
        let container = commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(20.0),
                width: Val::Px(200.0),
                height: Val::Px(CONTAINER_HEIGHT_PX),
                ..default()
            })
            .id();
        let area = spawn_scroll_list(&mut commands, COLORS, ());
        (container, area)
    };
    // Flush the spawns so `area`'s `ChildOf` (its root frame parent) is queryable.
    app.update();
    // Move the list's ROOT frame (the parent of `area`) under the short container.
    if let Some(root) = app.world().get::<ChildOf>(area).map(ChildOf::parent) {
        app.world_mut().entity_mut(container).add_child(root);
    }
    // Spawn the tall rows and parent them onto the scroll area.
    for i in 0..ROW_COUNT {
        let row = app
            .world_mut()
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(ROW_HEIGHT_PX),
                    min_height: Val::Px(ROW_HEIGHT_PX),
                    ..default()
                },
                BackgroundColor(Color::srgb((i as f32).mul_add(0.02, 0.2), 0.2, 0.3)),
            ))
            .id();
        app.world_mut().entity_mut(area).add_child(row);
    }
    for _ in 0..4 {
        app.update();
    }
    area
}

/// The scroll-area's computed content size (Y) and visible size (Y), in physical px.
fn content_and_visible_y(app: &App, area: Entity) -> Option<(f32, f32)> {
    let node = app.world().get::<ComputedNode>(area)?;
    Some((node.content_size().y, node.size().y))
}

/// The scroll-area's current [`ScrollPosition`] Y.
fn scroll_y(app: &App, area: Entity) -> f32 {
    app.world().get::<ScrollPosition>(area).map_or(0.0, |p| p.y)
}

/// Triggers the REAL engine [`Pointer<Scroll>`] entity-event on `area` — the exact event the
/// `bevy_picking` backend dispatches on a wheel turn — so the production
/// [`ScrollAreaPlugin`](bevy::ui_widgets::ScrollAreaPlugin) observer runs and clamps
/// [`ScrollPosition`]. `lines` is the wheel delta (positive = scroll DOWN the content).
///
/// The observer reads only `scroll.entity` + `scroll.{x,y,unit}`; it ignores the pointer
/// location + hit, so those are filled with inert placeholder values (a dummy window ref + a
/// placeholder camera) — this drives the real code path, not a reimplementation.
fn wheel_scroll(app: &mut App, area: Entity, lines: f32) {
    let Some(window_ref) = WindowRef::Entity(Entity::PLACEHOLDER).normalize(None) else {
        // `WindowRef::Entity(_).normalize(None)` always yields `Some`; this is unreachable.
        return;
    };
    let location = Location {
        target:   NormalizedRenderTarget::Window(window_ref),
        position: Vec2::ZERO,
    };
    let scroll = Scroll {
        unit:  MouseScrollUnit::Line,
        x:     0.0,
        // Negative-Y delta scrolls the content DOWN (the observer subtracts the delta).
        y:     -lines,
        hit:   HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        phase: TouchPhase::Moved,
    };
    let event = Pointer::new(PointerId::Mouse, location, scroll, area);
    app.world_mut().trigger(event);
    app.update();
}

/// C1 — a scroll list whose stacked rows are TALLER than its short container OVERFLOWS its
/// viewport, and a REAL wheel scroll (the production `ScrollAreaPlugin` observer) moves the
/// `ScrollPosition` down — clamped to the overflow, never unbounded.
///
/// Pin-discriminating: reverting `overflow: scroll_y` (or dropping `ScrollArea` / the
/// `ScrollAreaPlugin` wiring) makes the observer leave `ScrollPosition` at zero, failing the
/// "scrolled" assert; an unclamped scroll would exceed the overflow, failing the clamp
/// assert.
#[test]
fn tall_rows_overflow_and_wheel_scrolls_clamped() {
    let mut app = harness();
    let area = spawn_short_list_with_tall_rows(&mut app);

    // The content stack must be taller than the visible viewport — i.e. it overflows.
    let sizes = content_and_visible_y(&app, area);
    assert!(sizes.is_some(), "scroll area must have a computed layout");
    let (content_y, visible_y) = sizes.unwrap_or_default();
    assert!(
        content_y > visible_y,
        "rows taller than the container must overflow: content {content_y} <= visible {visible_y}",
    );

    // Precondition: unscrolled.
    assert!(
        scroll_y(&app, area).abs() < f32::EPSILON,
        "list starts at the top",
    );

    // Drive a real wheel scroll through the production observer.
    wheel_scroll(&mut app, area, 3.0);

    let after = scroll_y(&app, area);
    assert!(
        after > 0.0,
        "a real wheel scroll must move ScrollPosition down (got {after})",
    );

    // The overflow ceiling, in LOGICAL px (ComputedNode sizes are physical; the observer
    // clamps in logical units via `inverse_scale_factor`). Re-read the freshest computed
    // node and convert.
    let node = app.world().get::<ComputedNode>(area);
    assert!(node.is_some(), "scroll area must still be laid out");
    let max_y = node.map_or(0.0, |n| {
        ((n.content_size().y - n.size().y) * n.inverse_scale_factor).max(0.0)
    });
    assert!(
        after <= max_y + f32::EPSILON,
        "scroll must clamp to the overflow: {after} > max {max_y}",
    );

    // A huge scroll must not run away past the clamp.
    wheel_scroll(&mut app, area, 1000.0);
    let saturated = scroll_y(&app, area);
    assert!(
        saturated <= max_y + f32::EPSILON,
        "an over-scroll clamps to the overflow ceiling: {saturated} > max {max_y}",
    );
    assert!(
        saturated >= after,
        "scrolling further down never moves the position back up: {saturated} < {after}",
    );
}

/// C1 (structural) — the scroll list spawns exactly one [`ScrollbarThumb`] and, with content
/// overflowing, the engine sizes that thumb SHORTER than its track (visible/content ratio),
/// proving the scrollbar is wired to the area's overflow rather than filling the track.
///
/// Pin-discriminating: a thumb that is NOT wired to the area (no `Scrollbar.target`, or no
/// overflow) is sized to the full track length, failing the `thumb < track` assert.
#[test]
fn scrollbar_thumb_sized_from_overflow_ratio() {
    let mut app = harness();
    let area = spawn_short_list_with_tall_rows(&mut app);

    // Exactly one thumb exists.
    let thumbs: Vec<Entity> = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<ScrollbarThumb>>();
        q.iter(app.world()).collect()
    };
    assert_eq!(
        thumbs.len(),
        1,
        "the scroll list spawns one scrollbar thumb"
    );
    let thumb = thumbs[0];

    // The engine writes the thumb's ComputedNode after `ui_layout_system` (PostUpdate); a
    // few updates have run via `spawn_short_list_with_tall_rows`. The thumb's Y extent must
    // be SHORTER than the visible track height when the content overflows.
    let thumb_y = app
        .world()
        .get::<ComputedNode>(thumb)
        .map_or(0.0, |n| n.size().y);
    let track_y = content_and_visible_y(&app, area).map_or(0.0, |(_c, v)| v);
    assert!(thumb_y > 0.0, "the thumb has a computed size ({thumb_y})");
    assert!(
        thumb_y < track_y,
        "with overflow, the thumb is shorter than the track: thumb {thumb_y} >= track {track_y}",
    );
}
