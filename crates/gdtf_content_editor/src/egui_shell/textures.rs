//! The shell's PRE-PANEL **egui texture-id resolution** — split out of `shell.rs` at
//! the GTW-664 natural seam (module-layout bands): this block changes when a MODE's
//! texture surface changes (a new preview, a new registered sheet), the shell when the
//! PANEL layout does.
//!
//! Every id here must resolve BEFORE the exclusive `ctx_mut()` borrow the shell takes
//! (`image_id`/`add_image` borrow the contexts — `bevy_egui` 0.41), which is why the shell
//! calls [`resolve_panel_textures`] once, first, and threads the returned bundle into
//! the panel closures.

use bevy_egui::{EguiContexts, egui};

use super::{
    params::{PrefabParams, SpriteParams},
    sprite_form_ui,
    sprite_form_ui::PreviewTexture,
};
use crate::mode::EditorMode;

/// The egui texture ids the panels draw, resolved pre-`ctx_mut`. Each is [`None`] until
/// its asset registers/decodes — the consuming panel then draws its fallback.
pub(super) struct ResolvedTextures {
    /// The palette sprite sheet (the PREFAB palette rows + the TERRAIN/THEME pickers'
    /// thumbnails draw UV sub-rects over it).
    pub(super) sheet_id:       Option<egui::TextureId>,
    /// The offscreen prefab-preview render target (the PREFAB viewport's image).
    pub(super) preview_id:     Option<egui::TextureId>,
    /// The GTW-664 SPRITE-mode preview of the draft's base source image (the anchor
    /// section's crosshair canvas).
    pub(super) sprite_preview: Option<PreviewTexture>,
}

/// Resolve every egui texture id the panels need — the palette sheet + the prefab
/// render target (`image_id` lookups over the already-registered handles) and the
/// SPRITE-mode source preview (an idempotent load + `add_image` + measure through the
/// path-keyed cache — see
/// [`resolve_preview_texture`](sprite_form_ui::resolve_preview_texture)).
pub(super) fn resolve_panel_textures(
    contexts: &mut EguiContexts,
    mode: EditorMode,
    prefab: &PrefabParams,
    sprite_mode: &mut SpriteParams,
) -> ResolvedTextures {
    let sheet_id = prefab
        .atlas
        .as_deref()
        .and_then(|atlas| contexts.image_id(&atlas.image()));
    let preview_id = prefab
        .preview_target
        .as_deref()
        .and_then(|target| contexts.image_id(&target.image_handle()));
    let sprite_preview = sprite_form_ui::resolve_preview_texture(
        contexts,
        mode,
        sprite_mode.draft.as_deref(),
        sprite_mode.images.as_deref(),
        sprite_mode.asset_server.as_deref(),
        &mut sprite_mode.preview_cache,
    );
    ResolvedTextures {
        sheet_id,
        preview_id,
        sprite_preview,
    }
}
