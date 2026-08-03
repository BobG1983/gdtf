use bevy_egui::egui;
use gdtf_battle_presenter::source_parts;
use gdtf_content_families::sprites::SpriteDef;

use super::textures::SpriteTextures;

pub(crate) const THUMB_EDGE: f32 = 24.0;

const MISSING_THUMB: egui::Color32 = egui::Color32::from_rgb(200, 60, 200);

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

pub(crate) fn draw_thumb(ui: &mut egui::Ui, def: Option<&SpriteDef>, textures: &SpriteTextures) {
    match (def, thumb_image(def, textures)) {
        (_, Some(image)) => {
            ui.add(image);
        }
        (None, None) => {
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(THUMB_EDGE, THUMB_EDGE), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, MISSING_THUMB);
        }
        (Some(_), None) => {
            ui.allocate_space(egui::vec2(THUMB_EDGE, THUMB_EDGE));
        }
    }
}
