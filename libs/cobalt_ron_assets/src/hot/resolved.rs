//! Marker resource recording that a hot-RON chain has settled.

use core::marker::PhantomData;

use bevy::prelude::*;

/// Marks that the chain from `Spec` has settled for resource `T`.
///
/// Set once the asset applied, or its load failed and the fallback branch ran.
#[derive(Resource)]
pub struct HotRonResolved<Spec, T = Spec>(PhantomData<fn() -> (Spec, T)>)
where
    Spec: Send + Sync + 'static,
    T: Send + Sync + 'static;

impl<Spec, T> HotRonResolved<Spec, T>
where
    Spec: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    /// The marker for this chain.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<Spec, T> Default for HotRonResolved<Spec, T>
where
    Spec: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}
