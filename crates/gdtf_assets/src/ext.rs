use bevy::{app::App, asset::AssetApp, reflect::TypePath};
use serde::Deserialize;

use crate::{asset::RonAsset, loader::RonAssetLoader};

pub trait RonAssetAppExt {
                                    fn init_ron_asset<T>(&mut self) -> &mut Self
    where
        T: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static;

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
