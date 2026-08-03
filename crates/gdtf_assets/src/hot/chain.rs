use bevy::{asset::AssetServer, prelude::*, reflect::TypePath};

pub type HotRonMapFn<Spec, T> = fn(&Spec, &AssetServer) -> T;

pub type HotRonFallbackFn<T> = fn() -> T;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotRonPath(&'static str);

impl HotRonPath {
        #[must_use]
    pub const fn new(path: &'static str) -> Self {
        Self(path)
    }
}

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

        #[must_use]
    pub const fn path(&self) -> HotRonPath {
        self.path
    }

        #[must_use]
    pub const fn map(&self) -> HotRonMapFn<Spec, T> {
        self.map
    }

        #[must_use]
    pub const fn fallback(&self) -> Option<HotRonFallbackFn<T>> {
        self.fallback
    }

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
            #[must_use]
    pub const fn identity(path: &'static str) -> Self {
        Self::new(path, clone_payload::<Payload>, None)
    }

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
