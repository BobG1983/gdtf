//! The shell's PRE-PANEL **egui texture-id resolution** — split out of `shell.rs` at
//! the GTW-664 natural seam (module-layout bands): this block changes when a MODE's
//! texture surface changes (a new preview, a new registered sheet), the shell when the
//! PANEL layout does.
//!
//! Every id here must resolve BEFORE the exclusive `ctx_mut()` borrow the shell takes
//! (`image_id`/`add_image` borrow the contexts — `bevy_egui` 0.41), which is why the shell
//! calls [`resolve_panel_textures`] once, first, and threads the returned bundle into
//! the panel closures.
//!
//! GTW-665: the single palette `sheet_id` became the path-keyed [`SpriteTextures`] map —
//! one egui id + pixel dims per DISTINCT source-image path in the GTW-663
//! [`SpriteDefRegistry`], so every thumbnail consumer (prefab palette, theme library,
//! terrain picker) draws from the SAME def-driven resolution the battle renderer uses
//! (one resolution, two consumers). Loading + registration ride the SPRITE mode's
//! path-keyed [`SpritePreviewCache`](super::sprite_form_ui::SpritePreviewCache) (strong
//! handles) + `EguiContexts::add_image` (idempotent — the entry API returns the existing
//! id on a multipass re-run).

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

/// The path-keyed sprite source textures the thumbnail draws sample (GTW-665): each
/// DISTINCT base-source image path in the [`SpriteDefRegistry`] resolves to its egui
/// texture id + LOADED pixel dims. A path still decoding is simply absent — the thumb
/// draws its fallback until the next frame resolves it.
#[derive(Default)]
pub(super) struct SpriteTextures {
    /// One resolved `(egui id, pixel dims)` per registered source path.
    map: HashMap<SpriteImagePath, (egui::TextureId, UVec2)>,
}

impl SpriteTextures {
    /// The resolved egui texture id + pixel dims for `path`, if its image has decoded
    /// and registered.
    pub(super) fn get(&self, path: &SpriteImagePath) -> Option<(egui::TextureId, UVec2)> {
        self.map.get(path).copied()
    }
}

/// The egui texture ids the panels draw, resolved pre-`ctx_mut`. Each is [`None`] /
/// empty until its asset registers/decodes — the consuming panel then draws its fallback.
pub(super) struct ResolvedTextures {
    /// The per-path sprite source textures (GTW-665 — the PREFAB palette rows + the
    /// TERRAIN/THEME pickers' thumbnails draw def-resolved UV sub-rects over them).
    pub(super) sprites:        SpriteTextures,
    /// The offscreen prefab-preview render target (the PREFAB viewport's image).
    pub(super) preview_id:     Option<egui::TextureId>,
    /// The GTW-664 SPRITE-mode preview of the draft's base source image (the anchor
    /// section's crosshair canvas).
    pub(super) sprite_preview: Option<PreviewTexture>,
}

/// Resolve every egui texture id the panels need — the per-path sprite source map
/// (GTW-665), the prefab render target (an `image_id` lookup over the already-registered
/// handle), and the SPRITE-mode source preview (an idempotent load + `add_image` +
/// measure through the path-keyed cache — see
/// [`resolve_preview_texture`](sprite_form_ui::resolve_preview_texture)).
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

/// Build the per-path [`SpriteTextures`] map from the registry's DISTINCT base-source
/// paths: load each through the path-keyed cache (one strong handle per path — a repeat
/// call reuses it), measure the decoded dims from [`Assets<Image>`](bevy::asset::Assets),
/// and register with egui via `add_image` (idempotent). A path still decoding is skipped
/// this frame. Registry absent / no asset stack → the empty map (every thumb falls back).
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
