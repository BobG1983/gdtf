//! Path + map + optional fallback for a hot-RON resource.

use bevy::{asset::AssetServer, prelude::*, reflect::TypePath};

/// Map a loaded RON spec into the runtime resource.
pub type HotRonMapFn<Spec, T> = fn(&Spec, &AssetServer) -> T;

/// Build a fallback resource when the asset fails to load.
pub type HotRonFallbackFn<T> = fn() -> T;

/// Static asset path for a hot-RON chain.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotRonPath(&'static str);

impl HotRonPath {
    /// Wrap a path string.
    #[must_use]
    pub const fn new(path: &'static str) -> Self {
        Self(path)
    }
}

/// Configuration for loading and mapping a hot-RON resource.
#[derive(Resource, Debug, Clone)]
pub struct HotRonChain<Spec, T>
where
    Spec: TypePath + Send + Sync + 'static,
    T: Resource,
{
    path:     HotRonPath,
    map:      HotRonMapFn<Spec, T>,
    fallback: Option<HotRonFallbackFn<T>>,
}

impl<Spec, T> HotRonChain<Spec, T>
where
    Spec: TypePath + Send + Sync + 'static,
    T: Resource,
{
    /// Build a chain from path, map, and optional fallback.
    #[must_use]
    pub const fn new(
        path: &'static str,
        map: HotRonMapFn<Spec, T>,
        fallback: Option<HotRonFallbackFn<T>>,
    ) -> Self {
        Self {
            path: HotRonPath::new(path),
            map,
            fallback,
        }
    }

    /// Asset path.
    #[must_use]
    pub const fn path(&self) -> HotRonPath {
        self.path
    }

    /// Spec → resource map function.
    #[must_use]
    pub const fn map(&self) -> HotRonMapFn<Spec, T> {
        self.map
    }

    /// Optional load-failure fallback.
    #[must_use]
    pub const fn fallback(&self) -> Option<HotRonFallbackFn<T>> {
        self.fallback
    }

    /// Attach a fallback builder.
    #[must_use]
    pub const fn with_fallback(mut self, fallback: HotRonFallbackFn<T>) -> Self {
        self.fallback = Some(fallback);
        self
    }
}

impl<Payload> HotRonChain<Payload, Payload>
where
    Payload: Resource + Clone + TypePath,
{
    /// Chain that clones the RON payload into the resource.
    #[must_use]
    pub const fn identity(path: &'static str) -> Self {
        Self::new(path, clone_payload::<Payload>, None)
    }

    /// Identity chain with a fallback.
    #[must_use]
    pub const fn identity_with_fallback(
        path: &'static str,
        fallback: HotRonFallbackFn<Payload>,
    ) -> Self {
        Self::new(path, clone_payload::<Payload>, Some(fallback))
    }
}

fn clone_payload<Payload: Clone>(payload: &Payload, _asset_server: &AssetServer) -> Payload {
    payload.clone()
}
