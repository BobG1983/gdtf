//! The one-call app registration extension for hot-reloadable RON resources.

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

/// One-call registration of a HOT-RELOADABLE RON resource — the GTW-564 seam.
///
/// Each method wires ONE chain: the [`RonAsset<T>`](crate::RonAsset) loader,
/// the per-chain [`HotRonChain`] config, a `Startup` kick-off that stores the
/// persistent [`HotRonHandle`], an `Update` resolve gated to insert the runtime
/// resource exactly once, and an ungated `Update` redrive that re-derives it
/// live on a matching `Modified` event. A 13th hot resource is one payload type
/// plus one of these calls.
///
/// Registration SELF-GATES on an [`AssetServer`] being present (registering an
/// asset without one panics), so a `MinimalPlugins` headless app skips the
/// whole chain — no loader, no systems, no panic (`bevy-traps.md` #1) — the
/// `register_ron_tables` precedent this seam replaces.
pub trait HotRonAppExt {
    /// Registers a PLAIN hot-RON chain: the payload IS the runtime resource
    /// (`Deserialize` + `Resource`), cloned straight out of the loaded asset.
    ///
    /// No `Failed` fallback: a bad file leaves the resource absent (consumers
    /// stay gated on its presence), exactly as the per-site chains behaved.
    fn init_hot_ron_resource<T>(&mut self, path: &'static str) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + Clone + for<'de> Deserialize<'de> + TypePath;

    /// Registers a PLAIN hot-RON chain WITH the opt-in `Failed -> default`
    /// hook: a genuine [`LoadState::Failed`](bevy::asset::LoadState::Failed)
    /// inserts `fallback()` (with a `warn!` naming the path) so a
    /// presence-gated flow is never stranded — never while still loading (the
    /// situation-resolve precedent).
    fn init_hot_ron_resource_with_fallback<T>(
        &mut self,
        path: &'static str,
        fallback: HotRonFallbackFn<T>,
    ) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + Clone + for<'de> Deserialize<'de> + TypePath;

    /// Registers a MAPPED hot-RON chain: the deserialized payload `Spec`
    /// derives the runtime resource `T` through `map`, which runs WITH
    /// [`AssetServer`] access (so a resolution may load sub-assets — the theme
    /// resolves font keys through the server) on both the first resolve and
    /// every hot redrive.
    fn init_hot_ron_resource_mapped<Spec, T>(
        &mut self,
        path: &'static str,
        map: HotRonMapFn<Spec, T>,
    ) -> &mut Self
    where
        Spec: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
        T: Resource<Mutability = Mutable>;

    /// Registers a MAPPED hot-RON chain WITH the opt-in `Failed -> default`
    /// hook — [`init_hot_ron_resource_mapped`](Self::init_hot_ron_resource_mapped)
    /// plus [`init_hot_ron_resource_with_fallback`](Self::init_hot_ron_resource_with_fallback)'s
    /// failure behavior.
    fn init_hot_ron_resource_mapped_with_fallback<Spec, T>(
        &mut self,
        path: &'static str,
        map: HotRonMapFn<Spec, T>,
        fallback: HotRonFallbackFn<T>,
    ) -> &mut Self
    where
        Spec: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
        T: Resource<Mutability = Mutable>;

    /// Registers a hot-RON chain from an already-BUILT [`HotRonChain`] config —
    /// the seam a SECOND host of a published chain installs through (GTW-579).
    ///
    /// The chain OWNER exports its config constructor (the ui theme's
    /// `theme_hot_ron_chain`, the presenter's `tile_roles_hot_ron_chain`) so
    /// the path + map hook stay single-sourced; a second host (the editor)
    /// installs the WHOLE generic kick-off / resolve / redrive chain from that
    /// config — optionally re-configured with its own failure policy via
    /// [`HotRonChain::with_fallback`] — with no copied path constant, no copied
    /// resolve, and no hand-registered half-chain.
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

/// Installs one configured chain: the loader, the config resource, and the
/// kick-off / resolve / redrive triplet.
///
/// Trap (a): self-gates on the [`AssetServer`] (the `register_ron_tables`
/// precedent) — `init_ron_asset` panics at registration without the asset
/// machinery, so a `MinimalPlugins` app registers NOTHING and stays a no-op.
/// The resolve is gated `handle present AND resource not yet resolved` so it
/// inserts exactly once; the redrive is ungated and self-guards on its
/// `Option`al borrows (its `Messages<AssetEvent<…>>` buffer is registered by
/// the `init_ron_asset` in this same guard, so its `MessageReader` always
/// validates — `bevy-traps.md` #4).
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
