//! Bevy asset wrapper around a RON-deserialized payload.

use bevy::reflect::TypePath;

/// Bevy [`Asset`](bevy::asset::Asset) holding a typed RON value.
#[derive(bevy::asset::Asset, TypePath)]
pub struct RonAsset<T>(T)
where
    T: TypePath + Send + Sync + 'static;

impl<T> RonAsset<T>
where
    T: TypePath + Send + Sync + 'static,
{
    /// Wrap a deserialized value.
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
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
