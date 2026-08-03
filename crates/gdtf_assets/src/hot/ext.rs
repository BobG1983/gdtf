//! App extension to register hot-RON resources.

use bevy::{app::App, asset::AssetServer, ecs::component::Mutable, prelude::*, reflect::TypePath};
use serde::Deserialize;

use crate::{
    ext::RonAssetAppExt,
    hot::{
        chain::{HotRonChain, HotRonFallbackFn, HotRonMapFn},
        handle::HotRonHandle,
        systems::{kick_off_hot_ron_resource, redrive_hot_ron_resource, resolve_hot_ron_resource},
    },
};

/// Register hot-reloadable RON-backed resources on a Bevy app.
pub trait HotRonAppExt {
    /// Load `T` from RON at `path` (identity map).
    fn init_hot_ron_resource<T>(&mut self, path: &'static str) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + Clone + for<'de> Deserialize<'de> + TypePath;

    /// Identity load with a fallback if the asset fails.
    fn init_hot_ron_resource_with_fallback<T>(
        &mut self,
        path: &'static str,
        fallback: HotRonFallbackFn<T>,
    ) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + Clone + for<'de> Deserialize<'de> + TypePath;

    /// Load `Spec` from RON and map into resource `T`.
    fn init_hot_ron_resource_mapped<Spec, T>(
        &mut self,
        path: &'static str,
        map: HotRonMapFn<Spec, T>,
    ) -> &mut Self
    where
        Spec: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
        T: Resource<Mutability = Mutable>;

    /// Mapped load with fallback.
    fn init_hot_ron_resource_mapped_with_fallback<Spec, T>(
        &mut self,
        path: &'static str,
        map: HotRonMapFn<Spec, T>,
        fallback: HotRonFallbackFn<T>,
    ) -> &mut Self
    where
        Spec: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
        T: Resource<Mutability = Mutable>;

    /// Install a pre-built [`HotRonChain`].
    fn init_hot_ron_chain<Spec, T>(&mut self, chain: HotRonChain<Spec, T>) -> &mut Self
    where
        Spec: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
        T: Resource<Mutability = Mutable>;
}

impl HotRonAppExt for App {
    fn init_hot_ron_resource<T>(&mut self, path: &'static str) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + Clone + for<'de> Deserialize<'de> + TypePath,
    {
        install(self, HotRonChain::<T, T>::identity(path));
        self
    }

    fn init_hot_ron_resource_with_fallback<T>(
        &mut self,
        path: &'static str,
        fallback: HotRonFallbackFn<T>,
    ) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + Clone + for<'de> Deserialize<'de> + TypePath,
    {
        install(
            self,
            HotRonChain::<T, T>::identity_with_fallback(path, fallback),
        );
        self
    }

    fn init_hot_ron_resource_mapped<Spec, T>(
        &mut self,
        path: &'static str,
        map: HotRonMapFn<Spec, T>,
    ) -> &mut Self
    where
        Spec: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
        T: Resource<Mutability = Mutable>,
    {
        install(self, HotRonChain::new(path, map, None));
        self
    }

    fn init_hot_ron_resource_mapped_with_fallback<Spec, T>(
        &mut self,
        path: &'static str,
        map: HotRonMapFn<Spec, T>,
        fallback: HotRonFallbackFn<T>,
    ) -> &mut Self
    where
        Spec: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
        T: Resource<Mutability = Mutable>,
    {
        install(self, HotRonChain::new(path, map, Some(fallback)));
        self
    }

    fn init_hot_ron_chain<Spec, T>(&mut self, chain: HotRonChain<Spec, T>) -> &mut Self
    where
        Spec: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
        T: Resource<Mutability = Mutable>,
    {
        install(self, chain);
        self
    }
}

fn install<Spec, T>(app: &mut App, chain: HotRonChain<Spec, T>)
where
    Spec: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
    T: Resource<Mutability = Mutable>,
{
    if app.world().get_resource::<AssetServer>().is_none() {
        return;
    }
    app.init_ron_asset::<Spec>();
    app.insert_resource(chain);
    app.add_systems(Startup, kick_off_hot_ron_resource::<Spec, T>)
        .add_systems(
            Update,
            resolve_hot_ron_resource::<Spec, T>
                .run_if(resource_exists::<HotRonHandle<Spec>>.and_then(not(resource_exists::<T>))),
        )
        .add_systems(Update, redrive_hot_ron_resource::<Spec, T>);
}
