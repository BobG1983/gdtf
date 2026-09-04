//! Startup load, first resolve, and hot-reload redrive systems.

use bevy::{
    asset::{AssetEvent, AssetServer, Assets},
    ecs::component::Mutable,
    prelude::*,
    reflect::TypePath,
};

use crate::{
    asset::RonAsset,
    hot::{chain::HotRonChain, handle::HotRonHandle, resolved::HotRonResolved},
};

/// Load the RON asset and store its handle on startup.
pub fn kick_off_hot_ron_resource<Spec, T>(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    chain: Option<Res<HotRonChain<Spec, T>>>,
) where
    Spec: TypePath + Send + Sync + 'static,
    T: Resource,
{
    let (Some(asset_server), Some(chain)) = (asset_server, chain) else {
        return;
    };
    let handle = asset_server.load::<RonAsset<Spec>>(*chain.path());
    commands.insert_resource(HotRonHandle::<Spec>::new(handle));
}

/// Insert the mapped resource once the asset is ready (or run fallback on failure).
///
/// Marks the chain resolved on both branches; the fallback only fills an absent `T`.
pub fn resolve_hot_ron_resource<Spec, T>(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    chain: Option<Res<HotRonChain<Spec, T>>>,
    handle: Option<Res<HotRonHandle<Spec>>>,
    assets: Option<Res<Assets<RonAsset<Spec>>>>,
    existing: Option<Res<T>>,
) where
    Spec: TypePath + Send + Sync + 'static,
    T: Resource,
{
    let (Some(asset_server), Some(chain), Some(handle), Some(assets)) =
        (asset_server, chain, handle, assets)
    else {
        return;
    };

    if let Some(loaded) = assets.get(&**handle) {
        commands.insert_resource((chain.map())(&**loaded, &asset_server));
        commands.insert_resource(HotRonResolved::<Spec, T>::new());
        return;
    }

    let Some(fallback) = chain.fallback() else {
        return;
    };
    if !asset_server.load_state(&**handle).is_failed() {
        return;
    }
    commands.insert_resource(HotRonResolved::<Spec, T>::new());
    if existing.is_some() {
        warn!(
            "hot-RON: asset `{}` failed to load; keeping the {} already present",
            *chain.path(),
            short_type_name::<T>(),
        );
        return;
    }
    warn!(
        "hot-RON: asset `{}` failed to load; falling back to the default {}",
        *chain.path(),
        short_type_name::<T>(),
    );
    commands.insert_resource(fallback());
}

/// Re-map the resource when the underlying RON asset is modified.
pub fn redrive_hot_ron_resource<Spec, T>(
    mut events: MessageReader<AssetEvent<RonAsset<Spec>>>,
    asset_server: Option<Res<AssetServer>>,
    chain: Option<Res<HotRonChain<Spec, T>>>,
    handle: Option<Res<HotRonHandle<Spec>>>,
    assets: Option<Res<Assets<RonAsset<Spec>>>>,
    resource: Option<ResMut<T>>,
) where
    Spec: TypePath + Send + Sync + 'static,
    T: Resource<Mutability = Mutable>,
{
    let (Some(asset_server), Some(chain), Some(handle), Some(assets), Some(mut resource)) =
        (asset_server, chain, handle, assets, resource)
    else {
        events.clear();
        return;
    };

    let active_id = handle.id();
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { id } if *id == active_id));
    if !modified {
        return;
    }

    let Some(updated) = assets.get(&**handle) else {
        return;
    };
    *resource = (chain.map())(&**updated, &asset_server);
    info!(
        "hot-reload: re-derived {} from `{}`",
        short_type_name::<T>(),
        *chain.path(),
    );
}

/// Short type name for logs.
#[must_use]
pub fn short_type_name<T>() -> &'static str {
    let full = core::any::type_name::<T>();
    full.rsplit("::").next().unwrap_or(full)
}
