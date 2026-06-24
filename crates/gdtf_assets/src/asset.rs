//! The loaded-asset newtype produced by the RON loader.

use bevy::reflect::TypePath;

/// A loaded RON asset: a typed payload `T` deserialized from a loose `.ron` file.
///
/// This is the [`Asset`](bevy::asset::Asset) the
/// [`RonAssetLoader`](crate::RonAssetLoader) produces. It is a transparent
/// newtype over `T` and [`Deref`](std::ops::Deref)s to it, so a consumer that
/// holds a `RonAsset<MyType>` reads the `MyType` fields straight through.
///
/// `T` must be a `serde`-`Deserialize` value that is also `TypePath`, `Send`,
/// `Sync`, and `'static` (the bounds Bevy's asset system requires of any asset
/// payload). The loose-file path the asset was loaded from (e.g.
/// `theme/grimdark.ron`) is resolved by the host app's
/// `AssetServer` against its configured source root.
#[derive(bevy::asset::Asset, TypePath)]
pub struct RonAsset<T>(T)
where
    T: TypePath + Send + Sync + 'static;

impl<T> RonAsset<T>
where
    T: TypePath + Send + Sync + 'static,
{
    /// Wraps a deserialized payload `T` as the loaded asset.
    ///
    /// This is the ONLY way to construct a [`RonAsset`] from outside this module
    /// (the inner field is private per the no-bare-types newtype contract): the
    /// [`RonAssetLoader`](crate::RonAssetLoader) calls it to box the value it
    /// parsed from a `.ron` file, and the host app's loaders call it to seed an
    /// asset from a fallback value. `const fn` is sound here — `T` carries no
    /// bounds the wrapping needs, so the wrap is a trivial move.
    pub const fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T> core::ops::Deref for RonAsset<T>
where
    T: TypePath + Send + Sync + 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> core::ops::DerefMut for RonAsset<T>
where
    T: TypePath + Send + Sync + 'static,
{
    /// Mutable access to the payload — the only way to mutate it from outside
    /// this module now the inner field is private. The hot-reload path (and the
    /// tests that stand in for the file-watcher) overwrite an already-loaded
    /// asset's payload in place through this, e.g. `*asset = edited;`.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
