use bevy::{asset::Handle, prelude::*, reflect::TypePath};

use crate::asset::RonAsset;

#[derive(Resource, Deref, Debug, Clone)]
pub struct HotRonHandle<Spec>(Handle<RonAsset<Spec>>)
where
    Spec: TypePath + Send + Sync + 'static;

impl<Spec> HotRonHandle<Spec>
where
    Spec: TypePath + Send + Sync + 'static,
{
        #[must_use]
    pub const fn new(handle: Handle<RonAsset<Spec>>) -> Self {
        Self(handle)
    }
}
