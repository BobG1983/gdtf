//! The **shared egui sprite-thumbnail draw** (GTW-516 C3 / GTW-665) — the ONE code path
//! that turns a resolved [`SpriteDef`] into a fixed-size [`egui::Image`] over its source
//! texture's UV sub-rect.
//!
//! The GTW-515 PREFAB palette established the pattern (register the source image with egui
//! once, then draw a tile as an [`egui::Image`] UV sub-rect over its
//! [`egui::TextureId`]); GTW-665 re-anchored the RESOLUTION onto the sprite-def registry:
//! the prefab palette / theme library resolve a tile from a
//! [`TerrainUuid`](gdtf_battle_sim::terrain::def::TerrainUuid) (via
//! [`terrain_sprite_def`](crate::terrain_graphics::terrain_sprite_def)); the terrain
//! picker resolves a role key directly (via the presenter's
//! [`resolve_sprite`](gdtf_battle_presenter::resolve_sprite)). Both end at a
//! [`SpriteDef`], which this module turns into the pixels over the shell-resolved
//! [`SpriteTextures`](super::textures::SpriteTextures) map (one egui id + pixel dims per
//! DISTINCT source path in the registry) — either drawn as a plain [`egui::Image`]
//! ([`draw_thumb`], the palette-row thumbnail) or built as an [`egui::Image`] for a
//! caller-owned widget ([`thumb_image`], the picker's clickable/selectable sprite button).
//!
//! A row whose graphic resolves NO def paints the LOUD magenta missing square (the
//! GTW-665 C4 / Level-Rail precedent) — unresolved content flags instead of hiding.

use bevy_egui::egui;
use gdtf_battle_presenter::source_parts;
use gdtf_content_families::sprites::SpriteDef;

use super::textures::SpriteTextures;

/// The pixel edge of a sprite thumbnail. A framework layout const (the egui image is sized in
/// screen points, not a domain quantity), shared by the prefab palette row + the terrain picker
/// grid so both draw the tile at the same size.
pub(crate) const THUMB_EDGE: f32 = 24.0;

/// The LOUD missing-sprite thumbnail hue — the Level-Rail `FALLBACK_HUE` magenta the
/// GTW-665 C4 fallback standardizes on: a def-less graphic reads unmistakably
/// "unresolved", never a blank hole.
const MISSING_THUMB: egui::Color32 = egui::Color32::from_rgb(200, 60, 200);

/// Build the fixed-size ([`THUMB_EDGE`]) [`egui::Image`] for a resolved sprite def — its
/// source texture's [`egui::TextureId`] sampled over the def's UV sub-rect (a `Sheet`
/// source is its rect over the sheet dims; a `File` source is the whole image). Returns
/// [`None`] when the def is absent or its source texture has not yet decoded/registered
/// (the caller draws its fallback rather than panicking). `textures` is the shell's
/// pre-`ctx_mut` per-path resolution ([`SpriteTextures`]).
pub(crate) fn thumb_image(
    def: Option<&SpriteDef>,
    textures: &SpriteTextures,
) -> Option<egui::Image<'static>> {
    let def = def?;
    let (path, rect) = source_parts(&def.source);
    let (id, dims) = textures.get(path)?;
    if dims.x == 0 || dims.y == 0 {
        return None;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "source-image pixel coordinates are small (a sheet is hundreds of pixels); the \
                  f32 casts are exact within f32's 24-bit integer range"
    )]
    let uv = rect.map_or(
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        |rect| {
            let (w, h) = (dims.x as f32, dims.y as f32);
            let (x0, y0) = (*rect.x as f32 / w, *rect.y as f32 / h);
            egui::Rect::from_min_max(
                egui::pos2(x0, y0),
                egui::pos2(x0 + *rect.w as f32 / w, y0 + *rect.h as f32 / h),
            )
        },
    );
    Some(
        egui::Image::new(egui::load::SizedTexture::new(
            id,
            egui::vec2(THUMB_EDGE, THUMB_EDGE),
        ))
        .uv(uv),
    )
}

/// Draw a resolved sprite def as a plain fixed-size ([`THUMB_EDGE`]) [`egui::Image`]
/// (GTW-515 C4.2 / GTW-516 C3) — the palette-row thumbnail. A `None` def paints the LOUD
/// magenta missing square (GTW-665 C4 — an unresolved graphic flags instead of hiding);
/// a resolved def whose texture is still decoding allocates a blank fixed-size spacer,
/// so the caller's row layout stays stable either way (never a panic).
pub(crate) fn draw_thumb(ui: &mut egui::Ui, def: Option<&SpriteDef>, textures: &SpriteTextures) {
    match (def, thumb_image(def, textures)) {
        (_, Some(image)) => {
            ui.add(image);
        }
        (None, None) => {
            // Missing def — the loud magenta marker (the C4 / Level-Rail precedent).
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(THUMB_EDGE, THUMB_EDGE), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, MISSING_THUMB);
        }
        (Some(_), None) => {
            // Resolved def, texture still decoding — a quiet spacer keeps rows aligned.
            ui.allocate_space(egui::vec2(THUMB_EDGE, THUMB_EDGE));
        }
    }
}
