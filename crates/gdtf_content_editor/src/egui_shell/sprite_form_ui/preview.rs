//! The SPRITE form's **anchor section** (GTW-664 C2) — the VISUAL anchor affordance: a
//! scaled preview of the selected sprite with a crosshair marker at the authored
//! `(x, y)`, plus the two anchor drags.
//!
//! The preview rides the editor's established egui texture path (the GTW-515/516
//! `sprite_thumb` precedent): the source image is registered with egui ONCE (through
//! [`EguiContexts::add_image`], idempotent — the entry API returns the existing id on a
//! multipass re-run) and drawn as an [`egui::Image`] UV sub-rect over it; the marker is
//! painted OVER the image response rect, so it re-derives from the authored anchor every
//! pass (live by construction).

use bevy::{
    asset::{AssetServer, Assets},
    image::Image,
};
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};
use gdtf_content_families::sprites::{SpritePx, SpriteSource};

use super::cache::SpritePreviewCache;
use crate::{mode::EditorMode, sprite_form::SpriteDraft};

/// The preview box's largest on-screen edge, in egui points (a framework layout const —
/// the `THUMB_EDGE` precedent): a 16px tile scales up 10× so the anchor marker is
/// legible; a larger standalone image scales DOWN to fit.
const PREVIEW_MAX_EDGE: f32 = 160.0;

/// Half-length of the anchor crosshair's arms, in egui points (a framework layout
/// const).
const CROSSHAIR_ARM: f32 = 10.0;

/// The RESOLVED preview texture of the draft's base source image: the egui texture id
/// plus the loaded image's pixel dims (the [`SpritePx`] unit every sprite-def pixel
/// field shares). Built pre-`ctx_mut` by [`resolve_preview_texture`]; consumed by
/// [`anchor_section`] inside the panel closures.
pub(crate) struct PreviewTexture {
    /// The egui texture id the preview [`egui::Image`] samples.
    id:     egui::TextureId,
    /// The loaded source image's pixel width.
    width:  SpritePx,
    /// The loaded source image's pixel height.
    height: SpritePx,
}

impl PreviewTexture {
    /// The loaded source image's pixel width.
    pub(super) const fn width(&self) -> SpritePx {
        self.width
    }

    /// The loaded source image's pixel height.
    pub(super) const fn height(&self) -> SpritePx {
        self.height
    }
}

/// Resolve (load + egui-register + measure) the draft's BASE source image — called by
/// the shell BEFORE the exclusive `ctx_mut()` borrow (the `sheet_id` precedent: egui
/// texture ids must be resolved pre-pass). Returns [`None`] outside Sprite mode, while
/// the borrows are absent (state-scoped — bevy-traps #1), for an empty path, or while
/// the async image decode is still pending / failed — the section then draws a text
/// fallback instead of a texture.
///
/// Idempotent under the egui multipass re-run and across frames: the cache stores one
/// strong handle per path and [`EguiContexts::add_image`] returns the existing id for an
/// already-registered image.
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

/// Draw the ANCHOR section (GTW-664 C2): the two anchor drags (clamped to the sprite's
/// bounds where knowable) and the VISUAL affordance — the sprite's preview with a
/// crosshair marker at the authored `(x, y)`, re-derived from the draft every pass so it
/// reflects the live value.
///
/// The drag bounds layer the two knowledge sources: the MODEL's cheap bounds (a Sheet
/// rect's `w`/`h` — [`SpriteDraft::anchor_bounds`], which also clamps the commit) win;
/// a File source falls back to the LOADED image dims once the preview texture resolves
/// (the model invents no clamp for it — GTW-664 C4); with neither, the drag spans the
/// full `u32` type range (no invented bound).
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

/// Draw the scaled sprite preview ([`egui::Image`] over the source texture's UV
/// sub-rect) and paint the anchor crosshair over it at the authored point.
#[expect(
    clippy::cast_precision_loss,
    reason = "sprite-local pixel coordinates are small (a sprite is tens of pixels); the f32 \
              casts are exact within f32's 24-bit integer range (the sheet_uv precedent)"
)]
fn draw_preview_with_marker(ui: &mut egui::Ui, draft: &SpriteDraft, texture: &PreviewTexture) {
    let (image_w, image_h) = (*texture.width() as f32, *texture.height() as f32);
    if image_w <= 0.0 || image_h <= 0.0 {
        ui.label("(source image is empty)");
        return;
    }
    // The sprite's own pixel extent + the UV window it occupies on the source image: a
    // Sheet source is its rect cut; a File source is the whole image.
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
    // Fit-to-box scale — a 16px tile magnifies 10×, a big standalone image shrinks to
    // fit (capped so a degenerate 1px sprite cannot explode the layout).
    let scale = (PREVIEW_MAX_EDGE / sprite_w.max(sprite_h)).min(PREVIEW_MAX_EDGE);
    let display = egui::vec2(sprite_w * scale, sprite_h * scale);
    let response =
        ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id, display)).uv(uv));

    // The crosshair marker at the AUTHORED anchor, in on-screen points — re-derived
    // from the draft each pass, so a drag moves it live.
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
