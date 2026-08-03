//! `RailUiState` precedent): both maps are CONTENT-KEYED on the authored
use std::path::Path;

use bevy::{asset::AssetServer, image::Image, platform::collections::HashMap, prelude::Handle};
use gdtf_assets::WORKSPACE_ASSETS_ROOT;
use gdtf_content_families::sprites::SpriteImagePath;

#[derive(Default)]
pub(crate) struct SpritePreviewCache {
            textures:    HashMap<SpriteImagePath, Handle<Image>>,
                path_exists: HashMap<SpriteImagePath, bool>,
}

impl SpritePreviewCache {
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

                            pub(super) fn path_exists(&mut self, path: &SpriteImagePath) -> bool {
        *self.path_exists.entry(path.clone()).or_insert_with(|| {
            Path::new(WORKSPACE_ASSETS_ROOT)
                .join(path.as_str())
                .is_file()
        })
    }
}
