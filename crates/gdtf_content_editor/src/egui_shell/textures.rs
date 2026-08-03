use bevy::{asset::AssetServer, image::Image, math::UVec2, platform::collections::HashMap};
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};
use gdtf_battle_presenter::source_parts;
use gdtf_content_families::sprites::{SpriteDefRegistry, SpriteImagePath};

use super::{
    params::{PrefabParams, SpriteParams},
    sprite_form_ui,
    sprite_form_ui::PreviewTexture,
};
use crate::mode::EditorMode;

#[derive(Default)]
pub(super) struct SpriteTextures {
        map: HashMap<SpriteImagePath, (egui::TextureId, UVec2)>,
}

impl SpriteTextures {
            pub(super) fn get(&self, path: &SpriteImagePath) -> Option<(egui::TextureId, UVec2)> {
        self.map.get(path).copied()
    }
}

pub(super) struct ResolvedTextures {
            pub(super) sprites:        SpriteTextures,
        pub(super) preview_id:     Option<egui::TextureId>,
            pub(super) sprite_preview: Option<PreviewTexture>,
}

pub(super) fn resolve_panel_textures(
    contexts: &mut EguiContexts,
    mode: EditorMode,
    prefab: &PrefabParams,
    sprite_mode: &mut SpriteParams,
) -> ResolvedTextures {
    let preview_id = prefab
        .preview_target
        .as_deref()
        .and_then(|target| contexts.image_id(&target.image_handle()));
    let sprites = resolve_sprite_textures(
        contexts,
        sprite_mode.registry.as_deref(),
        sprite_mode.images.as_deref(),
        sprite_mode.asset_server.as_deref(),
        &mut sprite_mode.preview_cache,
    );
    let sprite_preview = sprite_form_ui::resolve_preview_texture(
        contexts,
        mode,
        sprite_mode.draft.as_deref(),
        sprite_mode.images.as_deref(),
        sprite_mode.asset_server.as_deref(),
        &mut sprite_mode.preview_cache,
    );
    ResolvedTextures {
        sprites,
        preview_id,
        sprite_preview,
    }
}

fn resolve_sprite_textures(
    contexts: &mut EguiContexts,
    registry: Option<&SpriteDefRegistry>,
    images: Option<&bevy::asset::Assets<Image>>,
    asset_server: Option<&AssetServer>,
    cache: &mut sprite_form_ui::SpritePreviewCache,
) -> SpriteTextures {
    let mut resolved = SpriteTextures::default();
    let (Some(registry), Some(images), Some(asset_server)) = (registry, images, asset_server)
    else {
        return resolved;
    };
    for (_name, def) in registry.defs() {
        let (path, _rect) = source_parts(&def.source);
        if resolved.map.contains_key(path) || path.trim().is_empty() {
            continue;
        }
        let handle = cache.handle(asset_server, path);
        let Some(size) = images.get(&handle).map(Image::size) else {
            continue;
        };
        let id = contexts.add_image(EguiTextureHandle::Strong(handle));
        resolved.map.insert(path.clone(), (id, size));
    }
    resolved
}
