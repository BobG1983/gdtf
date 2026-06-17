//! The generic RON [`AssetLoader`] and its extension-claim configuration.

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

/// A generic [`AssetLoader`] that reads a loose `.ron` file and deserializes it
/// into a [`RonAsset`] wrapping `T`.
///
/// One loader type serves every RON content type: register it per payload with
/// [`RonAssetAppExt::init_ron_asset`](crate::RonAssetAppExt::init_ron_asset).
/// The loader is stateless apart from the file extension(s) it claims;
/// [`Default`] constructs it claiming `ron`.
///
/// ## Extension disambiguation (GTW-257)
///
/// Bevy dispatches a `load_folder` (untyped, extension-based) load to the
/// LAST-registered loader for a file's extension. Because GDTF registers MANY
/// `RonAsset<T>` loaders that all claim `ron`, a `load_folder` of a `.ron`
/// directory is non-deterministically typed (it picks whichever `.ron` loader
/// registered last). A TYPED `load::<RonAsset<T>>(path)` is unaffected — it picks
/// the loader by asset TYPE — so single-file loads are fine; only folder loads are
/// ambiguous. To fold a folder of ONE concrete RON type, register that type's
/// loader with a DEDICATED extension via
/// [`RonAssetAppExt::init_ron_asset_with_extensions`](crate::RonAssetAppExt::init_ron_asset_with_extensions)
/// (e.g. `weapon.ron`) and name the files `*.weapon.ron`: Bevy's
/// [`AssetPath::get_full_extension`] then matches the dedicated extension FIRST,
/// so the dispatch is unambiguous regardless of registration order.
///
/// On `load`, it reads the entire file into a buffer and calls
/// `ron::de::from_bytes::<T>`. Any read or parse failure becomes a typed
/// [`RonLoadError`]; the happy path never panics or `unwrap`s.
///
/// It derives [`TypePath`] (required of every [`AssetLoader`]); the derive needs
/// `T: TypePath`, which every RON payload already satisfies via [`RonAsset`].
#[derive(TypePath)]
pub struct RonAssetLoader<T>
where
    T: TypePath,
{
    /// The file extension(s) this loader claims (e.g. `["ron"]`, or a dedicated
    /// `["weapon.ron"]` for an unambiguous folder load — see the type doc).
    extensions: Vec<&'static str>,
    /// Zero-sized type marker; the loader carries no other runtime state.
    _payload:   PhantomData<fn() -> T>,
}

impl<T> RonAssetLoader<T>
where
    T: TypePath,
{
    /// Build a loader claiming the given file `extensions` (the dedicated-extension
    /// path of the type doc). At least one extension should be supplied.
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
            .map_err(|err| RonLoadError::Read(ReadError(err)))?;
        let value = ron::de::from_bytes::<T>(&bytes)
            .map_err(|err| RonLoadError::Deserialize(RonDeError(err)))?;
        // GTW-146 hot-reload instrumentation: this `load` re-runs every time the
        // asset file-watcher detects an on-disk change, so a SECOND line here
        // after a save is the proof the watcher fired and the spec was rebuilt.
        bevy::log::info!(
            "RonAsset<{}> loaded/parsed ({} bytes)",
            T::short_type_path(),
            bytes.len(),
        );
        Ok(RonAsset(value))
    }

    fn extensions(&self) -> &[&str] {
        &self.extensions
    }
}
