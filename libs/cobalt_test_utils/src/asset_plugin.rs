//! Asset plugins for test apps, built with the file watcher off.
//!
//! Bevy's `file_watcher` feature makes an `AssetPlugin` watch its root, and the watcher
//! stats every file under that root as the app is built. A test that edits a file on disk
//! calls `AssetServer::reload` instead.

use std::path::Path;

use bevy::asset::AssetPlugin;

/// An `AssetPlugin` on Bevy's default asset root that does not watch for changes.
#[must_use]
pub fn unwatched_asset_plugin() -> AssetPlugin {
    AssetPlugin {
        watch_for_changes_override: Some(false),
        ..AssetPlugin::default()
    }
}

/// An `AssetPlugin` rooted at `root` that does not watch for changes.
#[must_use]
pub fn asset_plugin_at(root: &Path) -> AssetPlugin {
    let mut plugin = unwatched_asset_plugin();
    plugin.file_path = root.to_string_lossy().into_owned();
    plugin
}
