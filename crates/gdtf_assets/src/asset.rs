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
pub struct RonAsset<T>(pub T)
where
    T: TypePath + Send + Sync + 'static;

impl<T> core::ops::Deref for RonAsset<T>
where
    T: TypePath + Send + Sync + 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
