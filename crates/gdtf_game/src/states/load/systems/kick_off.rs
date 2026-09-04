use bevy::prelude::*;
use cobalt_ron_assets::{HotRonHandle, RonAsset};
use gdtf_content_families::{injuries::INJURIES_FOLDER, prefabs::PREFABS_FOLDER};
use gdtf_ui::theme::GdtfThemeSpec;

use crate::states::load::resources::{
    FontFolderHandle, InjuriesFolderHandle, LoadHandles, PrefabsFolderHandle,
};

const THEME_RON_PATH: &str = "core_tuning/ui_theme.tuning.ron";

const FONTS_FOLDER_PATH: &str = "fonts";

pub(in crate::states::load) fn kick_off_loads(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
) {
    let Some(asset_server) = asset_server else {
        return;
    };

    let theme = HotRonHandle::new(asset_server.load::<RonAsset<GdtfThemeSpec>>(THEME_RON_PATH));
    let fonts = FontFolderHandle::new(asset_server.load_folder(FONTS_FOLDER_PATH));
    let injuries = InjuriesFolderHandle::new(asset_server.load_folder(INJURIES_FOLDER));
    let prefabs = PrefabsFolderHandle::new(asset_server.load_folder(PREFABS_FOLDER));

    commands.insert_resource(LoadHandles {
        theme,
        fonts,
        injuries,
        prefabs,
    });
}
