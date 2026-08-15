use bevy_egui::egui;
use gdtf_battle_sim::{
    level::UuidThemeRegistry,
    metric::{CellLevel, Level},
    terrain::{def::TerrainDefRegistry, facing::TerrainFacing},
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

const SCROLL_PER_NOTCH: f32 = 50.0;

pub(crate) struct ViewportCtx<'a> {
    pub(crate) map:        &'a mut EditorMap,
    pub(crate) session:    &'a MapEditorSession,
    pub(crate) edit_level: &'a CurrentEditLevel,
    pub(crate) hovered:    &'a mut HoveredCell,
    pub(crate) zoom:       &'a mut CanvasZoom,
    pub(crate) pan:        &'a mut PreviewPan,
    pub(crate) registry:   Option<&'a TerrainDefRegistry>,
    pub(crate) themes:     Option<&'a UuidThemeRegistry>,
}

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

    if response.clicked()
        && let Some(uv) = response.interact_pointer_pos().map(|p| local_uv(p, rect))
    {
        paint_at_uv(ctx, uv, level);
    }

    if response.hovered() {
        let scroll = ui.ctx().input(|i| i.smooth_scroll_delta);
        if scroll.y.abs() > f32::EPSILON
            && let Some(uv) = hover_uv
        {
            let cursor_world = uv_to_world(bevy_uv(uv), scale, pan);
            let notches = scroll.y / SCROLL_PER_NOTCH;
            let outcome = cursor_anchored_zoom(*ctx.zoom, *ctx.pan, cursor_world, notches);
            *ctx.zoom = outcome.zoom();
            *ctx.pan = outcome.pan();
        }
    }

    if response.dragged_by(egui::PointerButton::Secondary) {
        let delta = response.drag_delta();
        if delta.length_sq() > f32::EPSILON {
            // view span at this scale is PREVIEW_VIEW_SPAN * scale across `rect.width()` points, so
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

fn paint_at_uv(ctx: &mut ViewportCtx<'_>, uv: egui::Vec2, level: Level) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let Some(tile) = ctx.session.selected_tile() else {
        return;
    };
    let cell = uv_to_cell(bevy_uv(uv), **ctx.zoom, ctx.pan.offset());
    let slot = CellLevel::new(cell, level);
    let placement = ProposedPlacement::new(slot, tile, TerrainFacing::default());
    apply_placement_with_pairing(
        ctx.map,
        registry,
        ctx.session.theme(),
        &placement,
        ctx.session.grid_size(),
    );
    let _ = ctx.themes;
}

fn local_uv(p: egui::Pos2, rect: egui::Rect) -> egui::Vec2 {
    (p - rect.min) / rect.size()
}

const fn bevy_uv(uv: egui::Vec2) -> bevy::math::Vec2 {
    bevy::math::Vec2::new(uv.x, uv.y)
}
