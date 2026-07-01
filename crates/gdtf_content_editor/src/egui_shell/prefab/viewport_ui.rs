//! The PREFAB-mode **render-to-texture viewport** (GTW-515 C4.3 / C4.4 / C4.7 / C4.8) — the
//! CENTRAL panel: the offscreen preview [`egui::Image`] widget, click-to-paint, the hover ghost
//! driver, wheel-zoom, and right-drag pan.
//!
//! ## The widget (C4.3)
//!
//! Draws the [`PreviewTarget`] image (rendered by the dedicated offscreen camera on the isolated
//! render layer — see [`preview`](crate::preview)) as an [`egui::Image`] over the panel's available
//! size, sensing click-and-drag. So the CENTRAL panel shows the live rendered prefab, not a
//! placeholder.
//!
//! ## Input → model, all set-to-target under multipass (bevy-traps #8 fact (b))
//!
//! egui is immediate-mode and its primary context runs MULTIPASS, so the UI closure may run twice
//! per frame. Every mutation here is therefore SAFE under a re-run:
//!
//! - HOVER (C4.4): each frame, write the [`HoveredCell`] model to the cell under the pointer (or
//!   clear it). Idempotent (a plain `set` to the same slot). The preview redraw draws the ghost.
//! - PAINT (C4.4): on a primary CLICK, run the shared [`apply_placement`] (ladder auto-clear /
//!   slab-seals-ladder reject — reused verbatim) for the clicked cell + the selected tile. A click
//!   is a discrete egui event that reports true on ONE pass of the frame; to be fully safe under a
//!   double-run we DEDUPE via the [`HoveredCell`]-slot no-op the model already provides (the map is
//!   a set keyed by slot, and `apply_placement` repaint-overwrites the SAME slot with the SAME
//!   tile — an idempotent write, so a double-apply is a no-op).
//! - WHEEL-ZOOM (C4.7): fold the scroll delta into the owned [`CanvasZoom`] via the PURE
//!   cursor-anchored math, computing an ABSOLUTE target from the CURRENT committed value — safe to
//!   set inside the closure (the same delta on both passes yields the same target).
//! - RIGHT-DRAG PAN (C4.8): fold the right-drag `drag_delta` into the owned [`PreviewPan`] as an
//!   ABSOLUTE target (`current + delta`) and SET it — never accumulate.
//!
//! The camera itself is driven from those owned targets by the once-per-frame
//! [`apply_preview_view`](crate::preview::target::apply_preview_view) OUTSIDE the closure, so the
//! camera mutation can never double-apply.

use bevy_egui::egui;
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    level::UuidThemeRegistry,
    metric::{CellLevel, Level},
    terrain::def::TerrainDefRegistry,
};

use crate::{
    canvas::{CanvasZoom, CurrentEditLevel},
    connector_pairing::apply_placement_with_pairing,
    editor_map::EditorMap,
    hovered_cell::HoveredCell,
    placement::ProposedPlacement,
    preview::{
        coords::{PREVIEW_VIEW_SPAN, uv_to_cell, uv_to_world},
        view::{PreviewPan, cursor_anchored_zoom},
    },
    session::MapEditorSession,
};

/// The scroll-delta magnitude that counts as one wheel "notch" for the cursor-anchored zoom — egui
/// reports `smooth_scroll_delta` in points; dividing by this yields a fractional notch count so the
/// zoom feels smooth. A framework tuning const (the wheel feel), not a domain value.
const SCROLL_PER_NOTCH: f32 = 50.0;

/// All the borrows the PREFAB viewport needs to draw + drive input. Bundled into one struct so the
/// shell threads a single `&mut` rather than a dozen params through its panel closure (the borrows
/// are distinct `SystemParam` slices the shell already holds). Not a domain value — a plumbing
/// aggregate of borrows.
pub(crate) struct ViewportCtx<'a> {
    /// The paintable map (mutated by click-to-paint).
    pub(crate) map:        &'a mut EditorMap,
    /// The authoring session (read for the theme / grid size / selected tile).
    pub(crate) session:    &'a MapEditorSession,
    /// The current edit storey the viewport paints on.
    pub(crate) edit_level: &'a CurrentEditLevel,
    /// The hovered-cell model (written each frame from the pointer).
    pub(crate) hovered:    &'a mut HoveredCell,
    /// The owned zoom target (folded from the wheel).
    pub(crate) zoom:       &'a mut CanvasZoom,
    /// The owned pan target (folded from right-drag).
    pub(crate) pan:        &'a mut PreviewPan,
    /// The terrain registry (for the placement legality classify).
    pub(crate) registry:   Option<&'a TerrainDefRegistry>,
    /// The theme registry (unused directly — placement resolves by UUID — kept for signature
    /// symmetry with the other panels + future theme-scoped rules).
    pub(crate) themes:     Option<&'a UuidThemeRegistry>,
    /// The presenter tile-role table (kept for signature symmetry — the preview redraw resolves
    /// sprites; the viewport itself needs no role lookup).
    pub(crate) roles:      Option<&'a TileRoles>,
}

/// Draw the PREFAB-mode render-to-texture viewport into the CENTRAL panel (GTW-515 C4.3) and drive
/// its input (C4.4 / C4.7 / C4.8).
///
/// `preview_id` is the egui texture id of the [`PreviewTarget`] image (resolved by the shell before
/// the draw); [`None`] means the target has not been registered yet (a placeholder shows). The
/// widget senses click-and-drag; the pointer's image-local UV drives the hover / paint mapping, the
/// scroll wheel drives the cursor-anchored zoom, and the RIGHT-drag drives the pan.
pub(crate) fn viewport_panel(
    ui: &mut egui::Ui,
    ctx: &mut ViewportCtx<'_>,
    preview_id: Option<egui::TextureId>,
) {
    let Some(preview_id) = preview_id else {
        ui.heading("Viewport");
        ui.label("Preparing preview render target…");
        return;
    };

    // C4.7 reset affordance — a thin toolbar row above the image restores the zoom + pan to their
    // defaults (identity zoom, origin pan). Set-to-target (never accumulate), so it is idempotent
    // under the egui multipass re-run.
    ui.horizontal(|ui| {
        if ui.button("Reset view").clicked() {
            *ctx.zoom = CanvasZoom::reset();
            *ctx.pan = PreviewPan::origin();
        }
        ui.label(format!("Zoom {:.2}x", **ctx.zoom));
    });

    let size = ui.available_size();
    let response = ui.add(
        egui::Image::new(egui::load::SizedTexture::new(preview_id, size))
            .sense(egui::Sense::click_and_drag()),
    );
    let rect = response.rect;

    // ── C4.4 hover: write the HoveredCell model from the pointer (or clear it). ──────────────────
    let level = ctx.edit_level.level();
    let scale = **ctx.zoom;
    let pan = ctx.pan.offset();
    let hover_uv = response
        .hover_pos()
        .filter(|_| rect.width() > 0.0 && rect.height() > 0.0)
        .map(|p| local_uv(p, rect));
    if let Some(uv) = hover_uv {
        let cell = uv_to_cell(bevy_uv(uv), scale, pan);
        ctx.hovered.set(cell, level);
    } else {
        ctx.hovered.clear();
    }

    // ── C4.4 paint: on a primary click, run the shared apply_placement for the clicked cell. ─────
    if response.clicked()
        && let Some(uv) = response.interact_pointer_pos().map(|p| local_uv(p, rect))
    {
        paint_at_uv(ctx, uv, level);
    }

    // ── C4.7 wheel-zoom (cursor-anchored), only while hovered. ───────────────────────────────────
    if response.hovered() {
        let scroll = ui.ctx().input(|i| i.smooth_scroll_delta);
        if scroll.y.abs() > f32::EPSILON
            && let Some(uv) = hover_uv
        {
            let cursor_world = uv_to_world(bevy_uv(uv), scale, pan);
            let notches = scroll.y / SCROLL_PER_NOTCH;
            let outcome = cursor_anchored_zoom(*ctx.zoom, *ctx.pan, cursor_world, notches);
            // SET absolute targets (never accumulate) — safe under the egui multipass re-run.
            *ctx.zoom = outcome.zoom();
            *ctx.pan = outcome.pan();
        }
    }

    // ── C4.8 right-drag pan: fold the right-drag delta into the owned pan as an ABSOLUTE target. ─
    if response.dragged_by(egui::PointerButton::Secondary) {
        let delta = response.drag_delta();
        if delta.length_sq() > f32::EPSILON {
            // A screen drag of `delta` points should move the VIEW by the world equivalent. The
            // view span at this scale is PREVIEW_VIEW_SPAN * scale across `rect.width()` points, so
            // one point = (span/width) world units. Dragging right should move the content right,
            // i.e. shift the camera centre LEFT — and egui's y grows down while world y grows up.
            let per_point = if rect.width() > 0.0 {
                PREVIEW_VIEW_SPAN * scale / rect.width()
            } else {
                0.0
            };
            let world_delta = bevy::math::Vec2::new(-delta.x * per_point, delta.y * per_point);
            *ctx.pan = PreviewPan::with_offset(pan + world_delta);
        }
    }
}

/// Run the shared placement for the cell under image-local UV `uv` on storey `level` (GTW-515
/// C4.4), auto-pairing a vertical connector (GTW-531). No-ops when no paint tile is selected or the
/// terrain registry is absent. The placement legality (ladder auto-clear, slab-seals-ladder reject,
/// out-of-bounds) is the shared GTW-430 predicate — reused verbatim, not re-implemented (C4.10);
/// placing an UP connector at `(x, y, N)` ALSO places its paired DOWN connector at `(x, y, N+1)`
/// via [`apply_placement_with_pairing`] (GTW-531 C2, fail-closed at the top storey).
fn paint_at_uv(ctx: &mut ViewportCtx<'_>, uv: egui::Vec2, level: Level) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let Some(tile) = ctx.session.selected_tile() else {
        return;
    };
    let cell = uv_to_cell(bevy_uv(uv), **ctx.zoom, ctx.pan.offset());
    let slot = CellLevel::new(cell, level);
    let placement = ProposedPlacement::new(slot, tile);
    // GTW-531: reuse the shared predicate (C4.10) AND auto-place the paired DOWN connector above an
    // UP connector. Idempotent under the egui multipass re-run: the map is a set keyed by slot and
    // apply_placement repaint-overwrites the SAME slots with the SAME tiles (a double-apply is a
    // no-op), so this is safe inside the immediate-mode closure (see the module docs).
    apply_placement_with_pairing(
        ctx.map,
        registry,
        ctx.session.theme(),
        &placement,
        ctx.session.grid_size(),
    );
    let _ = (ctx.themes, ctx.roles); // symmetry-only borrows (see the struct docs).
}

/// The image-local pointer position as an egui [`egui::Vec2`] UV in `[0,1]` (`(0,0)` = image
/// top-left) — the fraction of the image rect the pointer sits at.
fn local_uv(p: egui::Pos2, rect: egui::Rect) -> egui::Vec2 {
    (p - rect.min) / rect.size()
}

/// Convert an egui UV [`egui::Vec2`] to a bevy [`Vec2`](bevy::math::Vec2) for the coord mapping (the
/// two crates have distinct `Vec2` types).
const fn bevy_uv(uv: egui::Vec2) -> bevy::math::Vec2 {
    bevy::math::Vec2::new(uv.x, uv.y)
}
