//! Bevy asset loader for RON payloads into [`RonAsset`](crate::RonAsset).

use core::marker::PhantomData;

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    reflect::TypePath,
};
use serde::Deserialize;

use crate::{
    asset::RonAsset,
    error::{ReadError, RonDeError, RonLoadError},
};

/// Loads files into [`RonAsset<T>`] via `ron` deserialize.
#[derive(TypePath)]
pub struct RonAssetLoader<T>
where
    T: TypePath,
{
    extensions: Vec<&'static str>,
    _payload:   PhantomData<fn() -> T>,
}

impl<T> RonAssetLoader<T>
where
    T: TypePath,
{
    /// Build a loader that accepts the given extensions.
    #[must_use]
    pub fn with_extensions(extensions: Vec<&'static str>) -> Self {
        Self {
            extensions,
            _payload: PhantomData,
        }
    }
}

impl<T> Default for RonAssetLoader<T>
where
    T: TypePath,
{
    fn default() -> Self {
        Self {
            extensions: vec!["ron"],
            _payload:   PhantomData,
        }
    }
}

impl<T> AssetLoader for RonAssetLoader<T>
where
    T: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
{
    type Asset = RonAsset<T>;
    type Settings = ();
    type Error = RonLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(|err| RonLoadError::Read(ReadError::new(err)))?;
        let value = ron::de::from_bytes::<T>(&bytes)
            .map_err(|err| RonLoadError::Deserialize(RonDeError::new(err)))?;
        bevy::log::info!(
            "RonAsset<{}> loaded/parsed ({} bytes)",
            T::short_type_path(),
            bytes.len(),
        );
        Ok(RonAsset::new(value))
    }

    fn extensions(&self) -> &[&str] {
        &self.extensions
    }
}
