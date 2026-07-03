//! The one generic persistent handle every hot-RON chain holds.

use bevy::{asset::Handle, prelude::*, reflect::TypePath};

use crate::asset::RonAsset;

/// The persistent handle to the ACTIVE hot-reloadable RON asset whose payload is
/// `Spec` — the ONE handle newtype every hot-RON chain shares (GTW-564).
///
/// Replaces the twelve per-site handle newtypes (`FxTuningHandle`,
/// `KeybindsHandle`, `ActiveThemeHandle`, …) that each re-encoded the same
/// wrapper. A marker-parameterized generic newtype satisfies the no-bare-types
/// rule via its rule-4 plumbing carve-out (standing ruling `Q4`, 2026-07-02);
/// rule 5 still binds, so the inner [`Handle`] is PRIVATE and read through
/// [`Deref`] / built through [`new`](Self::new).
///
/// Inserted by the generic kick-off
/// ([`kick_off_hot_ron_resource`](crate::kick_off_hot_ron_resource)) — or by a
/// bespoke Load-side resolve for the chains whose kick-off stays hand-rolled
/// (the theme) — and NEVER removed, so it outlives any `Load`-scoped handle set:
/// the resolve reads it to poll the load, the redrive filters
/// [`AssetEvent`](bevy::asset::AssetEvent) ids against it on a hot edit, and
/// holding it keeps a STRONG reference so the asset stays loaded for the
/// file-watcher.
#[derive(Resource, Deref, Debug, Clone)]
pub struct HotRonHandle<Spec>(Handle<RonAsset<Spec>>)
where
    Spec: TypePath + Send + Sync + 'static;

impl<Spec> HotRonHandle<Spec>
where
    Spec: TypePath + Send + Sync + 'static,
{
    /// Wrap the active hot-RON asset handle.
    #[must_use]
    pub const fn new(handle: Handle<RonAsset<Spec>>) -> Self {
        Self(handle)
    }
}
