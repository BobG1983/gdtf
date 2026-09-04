//! scaled preview of the selected sprite with a crosshair marker at the authored
//! painted OVER the image response rect, so it re-derives from the authored anchor every
use bevy::{
    asset::{AssetServer, Assets},
    image::Image,
};
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};
use gdtf_content_families::sprites::{SpritePx, SpriteSource};

use super::cache::SpritePreviewCache;
use crate::{mode::EditorMode, sprite_form::SpriteDraft};

const PREVIEW_MAX_EDGE: f32 = 160.0;

const CROSSHAIR_ARM: f32 = 10.0;

pub(crate) struct PreviewTexture {
    id:     egui::TextureId,
    width:  SpritePx,
    height: SpritePx,
}

impl PreviewTexture {
    pub(super) const fn width(&self) -> SpritePx {
        self.width
    }

    pub(super) const fn height(&self) -> SpritePx {
        self.height
    }
}

pub(crate) fn resolve_preview_texture(
    contexts: &mut EguiContexts,
    mode: EditorMode,
    draft: Option<&SpriteDraft>,
    images: Option<&Assets<Image>>,
    asset_server: Option<&AssetServer>,
    cache: &mut SpritePreviewCache,
) -> Option<PreviewTexture> {
    if mode != EditorMode::Sprite {
        return None;
    }
    let (draft, images, asset_server) = draft
        .zip(images)
        .zip(asset_server)
        .map(|((draft, images), asset_server)| (draft, images, asset_server))?;
    let path = match &draft.def().source {
        SpriteSource::File(path) => path,
        SpriteSource::Sheet { sheet, .. } => sheet,
    };
    if path.trim().is_empty() {
        return None;
    }
    let handle = cache.handle(asset_server, path);
    let size = images.get(&handle)?.size();
    let id = contexts.add_image(EguiTextureHandle::Strong(handle));
    Some(PreviewTexture {
        id,
        width: SpritePx::new(size.x),
        height: SpritePx::new(size.y),
    })
}

pub(super) fn anchor_section(
    ui: &mut egui::Ui,
    draft: &mut SpriteDraft,
    texture: Option<&PreviewTexture>,
) {
    ui.heading("Anchor");
    ui.label("Ground-contact / pivot point, in sprite-local pixels from the top-left.");

    let bounds = draft
        .anchor_bounds()
        .or_else(|| texture.map(|texture| (texture.width(), texture.height())));
    let (max_x, max_y) = bounds.map_or((u32::MAX, u32::MAX), |(w, h)| (*w, *h));

    let anchor = draft.def().anchor;
    let (mut x, mut y) = (*anchor.x, *anchor.y);
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("x");
        changed |= ui
            .add(egui::DragValue::new(&mut x).range(0..=max_x))
            .changed();
        ui.label("y");
        changed |= ui
            .add(egui::DragValue::new(&mut y).range(0..=max_y))
            .changed();
    });
    if changed {
        draft.set_anchor(SpritePx::new(x), SpritePx::new(y));
    }

    match texture {
        Some(texture) => draw_preview_with_marker(ui, draft, texture),
        None => {
            ui.label("(sprite preview pending — set a readable source image)");
        }
    }
}

fn draw_preview_with_marker(ui: &mut egui::Ui, draft: &SpriteDraft, texture: &PreviewTexture) {
    let (image_w, image_h) = (*texture.width() as f32, *texture.height() as f32);
    if image_w <= 0.0 || image_h <= 0.0 {
        ui.label("(source image is empty)");
        return;
    }
    let (uv, sprite_w, sprite_h) = match &draft.def().source {
        SpriteSource::File(_) => (
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            image_w,
            image_h,
        ),
        SpriteSource::Sheet { rect, .. } => {
            let (x, y, w, h) = (
                *rect.x as f32,
                *rect.y as f32,
                *rect.w as f32,
                *rect.h as f32,
            );
            (
                egui::Rect::from_min_max(
                    egui::pos2(x / image_w, y / image_h),
                    egui::pos2((x + w) / image_w, (y + h) / image_h),
                ),
                w,
                h,
            )
        }
    };
    if sprite_w <= 0.0 || sprite_h <= 0.0 {
        ui.label("(rect has a zero extent — widen w/h to preview)");
        return;
    }
    let scale = (PREVIEW_MAX_EDGE / sprite_w.max(sprite_h)).min(PREVIEW_MAX_EDGE);
    let display = egui::vec2(sprite_w * scale, sprite_h * scale);
    let response =
        ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id, display)).uv(uv));

    let anchor = draft.def().anchor;
    let marker = response.rect.min + egui::vec2(*anchor.x as f32 * scale, *anchor.y as f32 * scale);
    let stroke = egui::Stroke::new(2.0, egui::Color32::RED);
    let painter = ui.painter();
    painter.line_segment(
        [
            marker - egui::vec2(CROSSHAIR_ARM, 0.0),
            marker + egui::vec2(CROSSHAIR_ARM, 0.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            marker - egui::vec2(0.0, CROSSHAIR_ARM),
            marker + egui::vec2(0.0, CROSSHAIR_ARM),
        ],
        stroke,
    );
    painter.circle_stroke(marker, CROSSHAIR_ARM / 2.0, stroke);
}
