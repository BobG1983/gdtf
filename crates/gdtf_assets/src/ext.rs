//! The one-call app registration extension for RON content types.

use bevy::{app::App, asset::AssetApp, reflect::TypePath};
use serde::Deserialize;

use crate::{asset::RonAsset, loader::RonAssetLoader};

/// One-call registration of the RON loader for a payload type `T`.
///
/// Bundles the two registrations a RON content type needs — the
/// `Assets<RonAsset<T>>` collection and the [`RonAssetLoader<T>`] — so a host
/// crate adds support for a new RON type in a single line.
pub trait RonAssetAppExt {
    /// Registers `Assets<RonAsset<T>>` and the [`RonAssetLoader<T>`] for `T`,
    /// claiming the default `ron` extension.
    ///
    /// After this, `asset_server.load::<RonAsset<T>>("path/to/file.ron")` loads
    /// and deserializes a loose `.ron` file into a `RonAsset<T>`. Suitable for the
    /// TYPED single-file loads (which pick the loader by asset type); for an
    /// unambiguous `load_folder` of one RON type use
    /// [`init_ron_asset_with_extensions`](RonAssetAppExt::init_ron_asset_with_extensions).
    fn init_ron_asset<T>(&mut self) -> &mut Self
    where
        T: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static;

    /// Registers `Assets<RonAsset<T>>` and a [`RonAssetLoader<T>`] for `T` claiming
    /// the given DEDICATED file `extensions` (e.g. `["weapon.ron"]`).
    ///
    /// Use this when a folder of ONE concrete RON type is loaded with
    /// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder): a
    /// dedicated extension makes Bevy's extension dispatch unambiguous regardless of
    /// how many other `ron` loaders are registered (GTW-257 — see
    /// [`RonAssetLoader`]'s type doc). Registers the loader INSTANCE (not via
    /// `Default`) so its `extensions()` carry the dedicated extension(s).
    fn init_ron_asset_with_extensions<T>(&mut self, extensions: Vec<&'static str>) -> &mut Self
    where
        T: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static;
}

impl RonAssetAppExt for App {
    fn init_ron_asset<T>(&mut self) -> &mut Self
    where
        T: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
    {
        self.init_asset::<RonAsset<T>>();
        self.init_asset_loader::<RonAssetLoader<T>>();
        self
    }

    fn init_ron_asset_with_extensions<T>(&mut self, extensions: Vec<&'static str>) -> &mut Self
    where
        T: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
    {
        self.init_asset::<RonAsset<T>>();
        self.register_asset_loader(RonAssetLoader::<T>::with_extensions(extensions));
        self
    }
}
