//! The SPRITE form's **path-keyed preview cache** (GTW-664 C2) — the view-local store
//! behind the preview texture and the source-path validity labels.
//!
//! Held as a `Local` of the shell system (view-local UI state — the GTW-595
//! `RailUiState` precedent): both maps are CONTENT-KEYED on the authored
//! [`SpriteImagePath`] (the Level-Rail content-signature pattern), so the cache is
//! self-correcting and
//! multipass-idempotent for free (bevy-traps #8 fact (b): the second pass probes the
//! same key and reuses the stored entry) and needs no state-scoped lifecycle. A path's
//! image is loaded ONCE per session ([`AssetServer::load`] returns the same handle for
//! the same path anyway); the STRONG handle stored here keeps the decoded image alive
//! for the egui draw. Editing the file on disk hot-reloads THROUGH the same handle, so
//! the preview follows without a cache eviction.

use std::path::Path;

use bevy::{asset::AssetServer, image::Image, platform::collections::HashMap, prelude::Handle};
use gdtf_assets::WORKSPACE_ASSETS_ROOT;
use gdtf_content_families::sprites::SpriteImagePath;

/// The SPRITE form's view-local preview/validation cache (see the [module doc](self)).
#[derive(Default)]
pub(crate) struct SpritePreviewCache {
    /// One STRONG image handle per referenced source path — loaded on first sight,
    /// reused every later frame (and kept alive for egui's texture extraction).
    textures:    HashMap<SpriteImagePath, Handle<Image>>,
    /// The memoized does-this-path-exist probe result per authored path — one
    /// filesystem stat per DISTINCT path per session (typing re-probes each edited
    /// prefix once; an unchanged field costs nothing per frame).
    path_exists: HashMap<SpriteImagePath, bool>,
}

impl SpritePreviewCache {
    /// The image handle for `path`, loading it through the `asset_server` on the first
    /// request (idempotent — later requests reuse the stored strong handle).
    /// `pub(crate)` since GTW-665: the shell's per-path [`SpriteTextures`] resolution
    /// (`egui_shell/textures.rs`) rides the same path-keyed store for every thumbnail
    /// source, not just the SPRITE mode's preview.
    pub(crate) fn handle(
        &mut self,
        asset_server: &AssetServer,
        path: &SpriteImagePath,
    ) -> Handle<Image> {
        self.textures
            .entry(path.clone())
            .or_insert_with(|| asset_server.load(path.as_str().to_owned()))
            .clone()
    }

    /// Whether `path` names an existing FILE under the workspace `assets/` root — the
    /// source-field validity probe (GTW-664 C2's "validated text path"). Checked against
    /// [`WORKSPACE_ASSETS_ROOT`] (the same one-owner root the Save button writes under,
    /// identical to the editor's `AssetPlugin.file_path`), memoized per distinct
    /// path. Editor-only tooling: the probe reads the developer's working tree, which
    /// is where the standalone editor runs by construction.
    pub(super) fn path_exists(&mut self, path: &SpriteImagePath) -> bool {
        *self.path_exists.entry(path.clone()).or_insert_with(|| {
            Path::new(WORKSPACE_ASSETS_ROOT)
                .join(path.as_str())
                .is_file()
        })
    }
}
