//! Handle resource for the active hot-RON asset.

use bevy::{asset::Handle, prelude::*, reflect::TypePath};

use crate::asset::RonAsset;

/// Live handle to the RON asset feeding a hot resource.
#[derive(Resource, Deref, Debug, Clone)]
pub struct HotRonHandle<Spec>(Handle<RonAsset<Spec>>)
where
    Spec: TypePath + Send + Sync + 'static;

impl<Spec> HotRonHandle<Spec>
where
    Spec: TypePath + Send + Sync + 'static,
{
    /// Wrap an asset handle.
    #[must_use]
    pub const fn new(handle: Handle<RonAsset<Spec>>) -> Self {
        Self(handle)
    }
}
