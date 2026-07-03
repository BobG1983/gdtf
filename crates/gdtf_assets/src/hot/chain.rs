//! The per-chain configuration a hot-RON chain's generic systems read.

use bevy::{asset::AssetServer, prelude::*, reflect::TypePath};

/// The map hook a hot-RON chain runs to derive its runtime resource `T` from the
/// deserialized payload `Spec`.
///
/// It runs WITH [`AssetServer`] access (not as a pure `fn`) because some
/// resolutions load sub-assets — the UI theme's `GdtfThemeSpec::resolve` loads
/// each font key through the server on BOTH the first resolve and every hot
/// redrive. Plain chains whose payload IS the resource use
/// [`HotRonChain::identity`], which clones the payload and ignores the server.
pub type HotRonMapFn<Spec, T> = fn(&Spec, &AssetServer) -> T;

/// The opt-in `Failed -> default` fallback hook of a hot-RON chain.
///
/// Runs ONLY when the chain's asset reaches a genuine
/// [`LoadState::Failed`](bevy::asset::LoadState::Failed) — never while the load
/// is still in flight (the situation-resolve precedent: inserting a default
/// early would clear a Load gate before the real data resolves). Chains that
/// register without one (the presenter role/tuning tables, keybinds) simply
/// leave the resource absent on a failed load, exactly as their per-site
/// resolves did.
pub type HotRonFallbackFn<T> = fn() -> T;

/// The loose-asset path of a hot-RON chain, relative to the asset source root.
///
/// A named newtype over the `&'static str` path (no-bare-types) so the chain
/// config names *what* the string is; read through [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotRonPath(&'static str);

impl HotRonPath {
    /// Wrap a loose-asset path (relative to the asset source root).
    #[must_use]
    pub const fn new(path: &'static str) -> Self {
        Self(path)
    }
}

/// The compile-time configuration of ONE hot-RON chain: the loose-asset path,
/// the [`HotRonMapFn`] deriving the runtime resource `T` from the payload
/// `Spec`, and the opt-in [`HotRonFallbackFn`].
///
/// One instance per chain, keyed by the `(Spec, T)` type pair — a strongly
/// typed per-chain resource, NOT a runtime descriptor table (registration stays
/// compile-time generic per type). Inserted by the
/// [`HotRonAppExt`](crate::HotRonAppExt) registration; a host that registers
/// only a HALF of a chain (the UI theme's redrive, the editor's reuse of the
/// theme / tile-role redrives against its own Load-side resolve) inserts it
/// directly from the chain owner's exported constructor.
#[derive(Resource, Debug, Clone)]
pub struct HotRonChain<Spec, T>
where
    Spec: TypePath + Send + Sync + 'static,
    T: Resource,
{
    /// The loose `.ron` path this chain loads and names in its reload logs.
    path:     HotRonPath,
    /// Derives the runtime resource from the deserialized payload.
    map:      HotRonMapFn<Spec, T>,
    /// The opt-in `Failed -> default` hook; `None` leaves the resource absent
    /// on a failed load.
    fallback: Option<HotRonFallbackFn<T>>,
}

impl<Spec, T> HotRonChain<Spec, T>
where
    Spec: TypePath + Send + Sync + 'static,
    T: Resource,
{
    /// Build a chain config from its path, map hook, and optional fallback.
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

    /// The chain's loose-asset path.
    #[must_use]
    pub const fn path(&self) -> HotRonPath {
        self.path
    }

    /// The chain's map hook.
    #[must_use]
    pub const fn map(&self) -> HotRonMapFn<Spec, T> {
        self.map
    }

    /// The chain's opt-in fallback hook, if registered.
    #[must_use]
    pub const fn fallback(&self) -> Option<HotRonFallbackFn<T>> {
        self.fallback
    }
}

impl<Payload> HotRonChain<Payload, Payload>
where
    Payload: Resource + Clone + TypePath,
{
    /// The plain-chain config: the payload IS the resource, so the map clones
    /// it straight out of the loaded asset (the server goes unused).
    #[must_use]
    pub const fn identity(path: &'static str) -> Self {
        Self::new(path, clone_payload::<Payload>, None)
    }

    /// The plain-chain config WITH the `Failed -> default` hook opted in.
    #[must_use]
    pub const fn identity_with_fallback(
        path: &'static str,
        fallback: HotRonFallbackFn<Payload>,
    ) -> Self {
        Self::new(path, clone_payload::<Payload>, Some(fallback))
    }
}

/// The identity map of a plain hot-RON chain: clone the deserialized payload as
/// the runtime resource, ignoring the [`AssetServer`].
fn clone_payload<Payload: Clone>(payload: &Payload, _asset_server: &AssetServer) -> Payload {
    payload.clone()
}
