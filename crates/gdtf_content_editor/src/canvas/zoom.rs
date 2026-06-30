//! Canvas **mouse-wheel zoom** (GTW-500 C3) — the [`CanvasZoom`] resource, the wheel-read system
//! (gated to cursor-over-canvas), and the cell/root re-layout system (cursor-anchored). The zoom
//! CHROME (the clickable `Zoom n%` readout + its refresh + the reset) lives in its own
//! [`zoom_chrome`](super::zoom_chrome) submodule (the size-cap split — C6).
//!
//! ## Zoom is CELL-SIZE RE-LAYOUT, never a transform scale (the CRITICAL ENGINE FACT)
//!
//! The canvas is PURE `bevy_ui` (a flex-wrapped grid of fixed-px [`CanvasCell`] nodes inside a
//! [`ScrollListArea`](gdtf_ui::ScrollListArea)), NOT a 2D world-space camera over sprites. So zoom
//! must NOT mutate the [`Camera2d`](bevy::prelude::Camera2d) transform (it would scale the whole UI
//! — bevy-traps #8), NOT [`UiTransform::scale`](bevy::ui::UiTransform) (it composites into the
//! rendered [`UiGlobalTransform`] but NOT into Taffy's [`ComputedNode`] geometry, so the scroll
//! area's overflow CLIPPING would diverge from the rendered size), and NOT
//! [`UiScale`](bevy::ui::UiScale) (it is GLOBAL — it would scale the whole app UI). Instead: a
//! [`CanvasZoom`] factor multiplies [`CANVAS_CELL_PX`](super::types::CANVAS_CELL_PX), and a system
//! rewrites the [`Node`] width/height on every [`CanvasCell`] and on the [`CanvasRoot`]. Taffy owns
//! the geometry, so the cells re-flow and the scroll overflow re-computes correctly, and
//! [`Interaction`] hover/click stays pixel-correct automatically.
//!
//! ## Wheel/scroll arbitration — PRIMARY: plain wheel over the canvas = ZOOM
//!
//! The user asked for mouse-wheel zoom literally, so the PRIMARY arbitration is plain wheel =
//! zoom over the canvas. The complication (the CRITICAL ENGINE FACT): the shared
//! `bevy_ui_widgets` [`ScrollArea`](bevy::ui_widgets::ScrollArea) consumes the wheel via a GLOBAL
//! `On<Pointer<Scroll>>` observer (`scrollarea_on_scroll`) that is independent of the
//! [`MouseWheel`] [`MessageReader`] stream — `scroll.propagate(false)` stops only the pointer-event
//! propagation, NOT the Message stream — and that one observer is shared by ALL three editor scroll
//! lists (palette / right panel / canvas), so it cannot be suppressed for the canvas alone without
//! re-implementing the shared widget. We OWN the wheel for zoom WITHOUT touching the shared observer
//! by exploiting the schedule order: the picking backend emits `Pointer<Scroll>` and runs the
//! observer in `PreUpdate`; this zoom system runs in `Update` and, on a zoom tick over the canvas,
//! OVERWRITES the canvas [`CanvasScrollArea`]'s [`ScrollPosition`] with the cursor-anchored target
//! it must set for cursor-anchored zoom (C3) anyway — discarding the observer's same-frame scroll delta
//! before `PostUpdate`'s `ui_layout_system` reads it. So a plain wheel tick over the canvas ZOOMS
//! and never visibly scrolls; a plain wheel tick elsewhere scrolls its list normally (this system
//! no-ops when the cursor is not over the canvas). This delivers the literal ask without the
//! ship-blocking "zooms AND scrolls on one tick" the CRITICAL ENGINE FACT forbids.

use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    ui::{ComputedNode, ScrollPosition, UiGlobalTransform},
    window::PrimaryWindow,
};

use super::types::{BOUNDARY_PX, CANVAS_CELL_PX, CanvasCell, CanvasRoot, CanvasScrollArea};

/// The minimum canvas zoom factor — cells shrink to a quarter of their base edge. A documented
/// framework layout const (the [`CANVAS_CELL_PX`] precedent), not a domain value.
const MIN_ZOOM: f32 = 0.25;

/// The maximum canvas zoom factor — cells grow to four times their base edge. A documented
/// framework layout const.
const MAX_ZOOM: f32 = 4.0;

/// How much one wheel notch multiplies the zoom — a 10% step per notch (a gentle, smooth zoom). A
/// documented framework layout const.
const ZOOM_STEP_PER_NOTCH: f32 = 1.1;

/// One [`MouseScrollUnit::Pixel`] of wheel delta expressed in notches — so a trackpad's
/// pixel-unit scroll zooms at a comparable rate to a mouse's line-unit notch. A documented
/// framework layout const (mirrors the engine's `SCROLL_UNIT_CONVERSION_FACTOR = 100`).
const PIXELS_PER_NOTCH: f32 = 100.0;

/// The canvas **zoom factor** (GTW-500 C3) — the multiplier on the base cell edge
/// [`CANVAS_CELL_PX`] the cell/root re-layout reads.
///
/// A named newtype over the bare `f32` factor (no-bare-types: a zoom factor is a domain value).
/// PRIVATE inner, derived [`Deref`]; the only mutators are [`CanvasZoom::scaled`] /
/// [`CanvasZoom::reset`], which CLAMP the factor to `[MIN_ZOOM, MAX_ZOOM]` so the cells can never
/// be sized to zero or absurdly large. A state-scoped [`Resource`] (inserted `OnEnter(Editing)`,
/// removed `OnExit(Editing)` — bevy-traps #1), seeded to `1.0` (the GTW-423 base scale).
#[derive(Resource, Deref, Clone, Copy, PartialEq, Debug)]
pub struct CanvasZoom(f32);

impl CanvasZoom {
    /// The unzoomed factor — cells at their base [`CANVAS_CELL_PX`] edge (the editor's open state).
    #[must_use]
    pub const fn identity() -> Self {
        Self(1.0)
    }

    /// This factor MULTIPLIED by `factor`, CLAMPED to `[MIN_ZOOM, MAX_ZOOM]` (C3 — a sensible
    /// range). A multiply (not an add) makes each wheel notch a constant proportional step.
    #[must_use]
    pub fn scaled(self, factor: f32) -> Self {
        Self((self.0 * factor).clamp(MIN_ZOOM, MAX_ZOOM))
    }

    /// Reset to the unzoomed factor — the zoom-reset hotkey/button target (C3).
    #[must_use]
    pub const fn reset() -> Self {
        Self::identity()
    }

    /// The cell EDGE in logical pixels at this zoom — [`CANVAS_CELL_PX`] times the factor. The
    /// value the cell/root re-layout writes into each [`CanvasCell`] [`Node`] width/height.
    #[must_use]
    pub fn cell_px(self) -> f32 {
        CANVAS_CELL_PX * self.0
    }

    /// The raw zoom factor — for the cursor-anchor ratio (`new / old`).
    #[must_use]
    const fn factor(self) -> f32 {
        self.0
    }
}

/// The cell-count span the [`CanvasRoot`] grid wraps — `width × height` cells — carried on the
/// root so the zoom re-layout can recompute the row width at the new cell edge WITHOUT re-reading
/// the [`CanvasExtent`](super::types::CanvasExtent) (which would force a wider root query).
///
/// A named newtype-bearing component (no-bare-types): the spans are the grid's cell counts the
/// layout multiplies by the cell edge, stored as private `u16`s read through an accessor.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct CanvasExtentSpan {
    /// The grid's x span in cells.
    width: u16,
}

impl CanvasExtentSpan {
    /// Build a span from the grid's x cell count (the row width the flex-wrap container needs).
    #[must_use]
    pub(crate) const fn new(width: u16) -> Self {
        Self { width }
    }

    /// The grid container's pixel width at `cell_px` — `width` cells across plus the two boundary
    /// borders (mirrors [`grid_container_node`](super::types::grid_container_node)).
    fn row_width(self, cell_px: f32) -> f32 {
        2.0f32.mul_add(BOUNDARY_PX, cell_px * f32::from(self.width))
    }
}

/// `Update` (in `Editing`): a mouse-wheel turn OVER the canvas zooms the [`CanvasZoom`] (C3).
///
/// Reads the [`MouseWheel`] [`MessageReader`] (NOT an `EventReader` — bevy-traps #4), gated to
/// cursor-over-canvas: the [`PrimaryWindow`]'s
/// [`physical_cursor_position`](bevy::window::Window::physical_cursor_position) (physical px,
/// matching [`UiGlobalTransform`]) must satisfy [`ComputedNode::contains_point`] on the CANVAS's
/// [`CanvasScrollArea`] viewport. The gate is the canvas's scroll area SPECIFICALLY (not a bare
/// [`ScrollListArea`](gdtf_ui::ScrollListArea)): the editor spawns three scroll areas (palette /
/// canvas / right panel), so a bare-`ScrollListArea` gate would zoom the canvas on a wheel turn over
/// the palette or the right panel. The wheel delta (handling [`MouseScrollUnit::Line`] vs [`Pixel`])
/// multiplies the zoom by [`ZOOM_STEP_PER_NOTCH`] per notch, with
/// [`set_if_neq`](DetectChangesMut::set_if_neq) so an at-the-limit notch does not re-trigger the
/// re-layout. Per the wheel/scroll arbitration (module doc), the same-frame observer scroll is
/// overwritten by the cursor-anchored [`apply_canvas_zoom`] re-layout, so the canvas zooms without
/// scrolling. Guarded on the optional state-scoped [`CanvasZoom`] (bevy-traps #1).
pub(crate) fn read_zoom_wheel(
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    area: Query<(&ComputedNode, &UiGlobalTransform), With<CanvasScrollArea>>,
    zoom: Option<ResMut<CanvasZoom>>,
) {
    let Some(mut zoom) = zoom else {
        return;
    };
    // Accumulate this frame's notches first, then act once — so multiple events in one frame
    // compose into a single zoom write.
    let mut notches = 0.0_f32;
    for event in wheel.read() {
        notches += match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / PIXELS_PER_NOTCH,
        };
    }
    if notches.abs() < f32::EPSILON {
        return;
    }
    let Some(cursor) = windows
        .iter()
        .find_map(bevy::window::Window::physical_cursor_position)
    else {
        return;
    };
    // Gate to cursor-over-canvas: the cursor must be inside the scroll viewport that clips the
    // canvas (so a wheel turn elsewhere never zooms the canvas).
    let over_canvas = area
        .iter()
        .any(|(node, transform)| node.contains_point(*transform, cursor));
    if !over_canvas {
        return;
    }
    zoom.set_if_neq(zoom.scaled(ZOOM_STEP_PER_NOTCH.powf(notches)));
}

/// The [`CanvasRoot`] grid container's query item in [`apply_canvas_zoom`] — its span (to recompute
/// the row width) and its [`Node`] (to write the new width). A named alias to keep the system
/// signature under clippy's `type_complexity` gate.
type RootSpanNode = (&'static CanvasExtentSpan, &'static mut Node);

/// The [`CanvasRoot`] query filter in [`apply_canvas_zoom`] — the root, disjoint from the cells so
/// the two `&mut Node` queries can borrow at once (bevy-traps #7). A named alias for the
/// `type_complexity` gate.
type RootFilter = (With<CanvasRoot>, Without<CanvasCell>);

/// The canvas [`CanvasScrollArea`]'s query item in [`apply_canvas_zoom`] — its computed size +
/// global transform (the cursor-anchor math) and its mutable [`ScrollPosition`] (the re-anchor). A
/// named alias to keep the system signature under clippy's `type_complexity` gate.
type AreaScroll = (
    &'static ComputedNode,
    &'static UiGlobalTransform,
    &'static mut ScrollPosition,
);

/// The CANVAS scroll-area query filter in [`apply_canvas_zoom`] — the canvas's own
/// [`CanvasScrollArea`] (NOT a bare [`ScrollListArea`](gdtf_ui::ScrollListArea), of which the editor
/// has three: palette / canvas / right panel), disjoint from the cells AND the root so all three
/// `&mut`/`&` `Node`/`ScrollPosition` queries borrow at once (bevy-traps #7). The discriminator is
/// load-bearing: the cursor-anchored re-scroll OVERWRITES `ScrollPosition`, so applying it to the
/// other two lists would clobber their independent scroll on every canvas zoom tick. A named alias
/// for the `type_complexity` gate.
type AreaFilter = (
    With<CanvasScrollArea>,
    Without<CanvasCell>,
    Without<CanvasRoot>,
);

/// `Update` (in `Editing`): re-lay-out the canvas cells
/// + root to the new [`CanvasZoom`] and re-anchor the scroll position under the cursor (C3).
///
/// Rewrites each [`CanvasCell`] [`Node`] width/height to [`CanvasZoom::cell_px`] and the
/// [`CanvasRoot`] width to the new total span (NOT [`UiTransform::scale`], NOT [`UiScale`], NOT a
/// camera scale — the CRITICAL ENGINE FACT). Then CURSOR-ANCHORED zoom (C3): the content point
/// under the cursor is kept put by scaling the [`ScrollListArea`]'s [`ScrollPosition`] about the
/// cursor by the exact factor change `new / old` (tracked in a [`Local`] of the previous
/// [`CanvasZoom`] — content scales linearly with the zoom factor, so the ratio is exact and needs no
/// stale geometry read), clamped to the new `[0, max]`. The cursor/transform math is in PHYSICAL px (matching
/// [`ComputedNode::size`] + [`physical_cursor_position`](bevy::window::Window::physical_cursor_position));
/// the scroll position is LOGICAL px, so the physical cursor delta is converted via the area's
/// [`inverse_scale_factor`](bevy::ui::ComputedNode::inverse_scale_factor). This also OWNS the wheel
/// against the shared scroll observer (module doc): overwriting `ScrollPosition` here discards the
/// observer's same-frame scroll delta. Cell/root sizes are written with a `!=` guard so an
/// unchanged geometry write does not dirty the layout needlessly.
pub(crate) fn apply_canvas_zoom(
    zoom: Option<Res<CanvasZoom>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cells: Query<&mut Node, With<CanvasCell>>,
    mut root: Query<RootSpanNode, RootFilter>,
    rebuilt: Query<(), Added<CanvasRoot>>,
    mut area: Query<AreaScroll, AreaFilter>,
    mut prev_zoom: Local<Option<CanvasZoom>>,
) {
    let Some(zoom) = zoom else {
        return;
    };
    // Re-size on a zoom change OR a canvas REBUILD (a size/theme change respawns the cells at the
    // base edge — `sync_canvas` does not know the zoom; this re-applies it so a rebuild keeps the
    // current zoom). Nothing to do otherwise.
    if !zoom.is_changed() && rebuilt.is_empty() {
        return;
    }
    let cell_px = zoom.cell_px();

    // Re-size every cell to the new edge.
    for mut node in &mut cells {
        let want = Val::Px(cell_px);
        if node.width != want {
            node.width = want;
        }
        if node.height != want {
            node.height = want;
        }
    }
    // Re-size the root grid container so it wraps exactly `width` cells across at the new edge.
    for (span, mut node) in &mut root {
        let want = Val::Px(span.row_width(cell_px));
        if node.width != want {
            node.width = want;
        }
    }

    // The exact per-axis ratio is the same on both axes (uniform zoom): new factor / old factor.
    // On the very first run (or after a rebuild) there is no previous zoom — treat it as the
    // current one (no re-scroll), then record it.
    let old = prev_zoom.unwrap_or(*zoom).factor();
    *prev_zoom = Some(*zoom);
    let ratio = if old.abs() < f32::EPSILON {
        1.0
    } else {
        zoom.factor() / old
    };
    if (ratio - 1.0).abs() < f32::EPSILON {
        return;
    }

    // Cursor-anchored re-scroll: keep the content point under the cursor fixed.
    let Some(cursor) = windows
        .iter()
        .find_map(bevy::window::Window::physical_cursor_position)
    else {
        return;
    };
    for (node, transform, mut scroll) in &mut area {
        let inv = node.inverse_scale_factor();
        // `UiGlobalTransform.translation` is the node CENTRE (physical px); the viewport top-left is
        // centre minus half the (physical) size. Cursor offset within the viewport, in logical px.
        let top_left = transform.translation - node.size() * 0.5;
        let cursor_in_view = (cursor - top_left) * inv;
        // The content point under the cursor BEFORE the re-layout (logical px) is `scroll + cursor`.
        // After scaling the content by `ratio`, that point moves to `(scroll + cursor) * ratio`;
        // keep it under the cursor by re-deriving the scroll offset, then clamp to the new extent.
        let new_scroll = (scroll.0 + cursor_in_view) * ratio - cursor_in_view;
        let viewport = node.size() * inv;
        let new_content = scroll_content_extent(node, inv, ratio);
        let max = (new_content - viewport).max(Vec2::ZERO);
        scroll.0 = new_scroll.clamp(Vec2::ZERO, max);
    }
}

/// The area's content extent (logical px) AFTER the zoom — the pre-zoom
/// [`content_size`](bevy::ui::ComputedNode::content_size) (still the prior layout this frame, since
/// the cell [`Node`] writes apply in `PostUpdate`) scaled by the zoom `ratio`.
fn scroll_content_extent(area: &ComputedNode, inv: f32, ratio: f32) -> Vec2 {
    area.content_size() * inv * ratio
}
