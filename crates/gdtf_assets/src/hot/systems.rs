//! The ONE kick-off / resolve / redrive system triplet every hot-RON chain runs.
//!
//! Before GTW-564 this quadruplet (handle newtype + the three systems) was
//! hand-stamped once per chain across four crates, each copy re-encoding the
//! same drain body and the same four traps. The traps now live here exactly
//! once:
//!
//! - **(a) headless inertness** — the kick-off takes `Option<Res<AssetServer>>`
//!   (and registration itself self-gates on the server — see
//!   [`HotRonAppExt`](crate::HotRonAppExt)), so a `MinimalPlugins` app stays a
//!   no-op (`bevy-traps.md` #1).
//! - **(b) pre-resolve drain** — the redrive `events.clear()`s its reader while
//!   any resource it needs is absent, so a stale event never lingers and
//!   re-fires once the resources arrive.
//! - **(c) once-per-frame `Modified` id-filter** — the redrive acts only on a
//!   [`Modified`](bevy::asset::AssetEvent::Modified) whose id matches the
//!   active [`HotRonHandle`], and re-derives at most ONCE per frame however
//!   many events arrive (a single re-derive from the latest in-memory value
//!   covers them all).
//! - **(d) `ResMut`-overwrite change contract** — the redrive overwrites the
//!   resident resource THROUGH `ResMut`, marking it changed so change-driven
//!   consumers (the theme repaint, the sim's stat re-derive, the terrain
//!   redraw) react the same frame.

use bevy::{
    asset::{AssetEvent, AssetServer, Assets},
    ecs::component::Mutable,
    prelude::*,
    reflect::TypePath,
};

use crate::{
    asset::RonAsset,
    hot::{chain::HotRonChain, handle::HotRonHandle},
};

/// `Startup`: kick off a hot-RON chain's load, storing its persistent
/// [`HotRonHandle`].
///
/// Loads the chain's configured path as a [`RonAsset<Spec>`] through the
/// generic loader and inserts the [`HotRonHandle<Spec>`] the resolve polls and
/// the redrive filters against. Takes `Option<Res<AssetServer>>` so a headless
/// app with no [`AssetServer`] no-ops rather than panicking (trap (a),
/// `bevy-traps.md` #1) — belt-and-braces on top of the registration self-gate.
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

/// `Update` (gated until `T` is resolved): resolve the loaded RON into the
/// chain's runtime resource `T` — inserting it exactly ONCE.
///
/// Once the [`RonAsset<Spec>`] settles into its `Assets` collection (a
/// transient loaded-but-not-yet-in-collection frame simply retries next pass),
/// it derives `T` through the chain's [map hook](HotRonChain::map) — which runs
/// WITH [`AssetServer`] access, so a resolution may load sub-assets (fonts) —
/// and inserts it. Registered
/// `run_if(resource_exists::<HotRonHandle<Spec>> AND not(resource_exists::<T>))`
/// so it inserts once and NEVER re-publishes over live data; the live re-derive
/// is [`redrive_hot_ron_resource`].
///
/// If the chain opted into a [fallback](HotRonChain::fallback), a load that
/// reaches a genuine [`LoadState::Failed`](bevy::asset::LoadState::Failed)
/// `warn!`s naming the path and inserts the fallback default — NEVER while the
/// load is still in flight (the situation-resolve precedent), so a gate keyed
/// on `T`'s presence still waits for the real data. Without a fallback a failed
/// load leaves `T` absent, exactly as the per-site chains did.
pub fn resolve_hot_ron_resource<Spec, T>(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    chain: Option<Res<HotRonChain<Spec, T>>>,
    handle: Option<Res<HotRonHandle<Spec>>>,
    assets: Option<Res<Assets<RonAsset<Spec>>>>,
) where
    Spec: TypePath + Send + Sync + 'static,
    T: Resource,
{
    let (Some(asset_server), Some(chain), Some(handle), Some(assets)) =
        (asset_server, chain, handle, assets)
    else {
        return;
    };

    // Success path: the payload is in the collection — derive + insert ONCE
    // (the run-condition retires this system the moment T exists).
    if let Some(loaded) = assets.get(&**handle) {
        commands.insert_resource((chain.map())(&**loaded, &asset_server));
        return;
    }

    // Failure path — ONLY for chains that opted in, and ONLY on a genuine
    // terminal Failed (never mid-load): fall back to the chain's default so a
    // presence-gated flow (the Load gate) is never stranded by a bad file.
    let Some(fallback) = chain.fallback() else {
        return;
    };
    if asset_server.load_state(&**handle).is_failed() {
        warn!(
            "hot-RON: asset `{}` failed to load; falling back to the default {}",
            *chain.path(),
            short_type_name::<T>(),
        );
        commands.insert_resource(fallback());
    }
}

/// `Update` (ungated; self-gates on its [`Option`] borrows): re-derive the
/// chain's resident resource `T` in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) — the LIVE
/// hot-reload.
///
/// Reads the [`MessageReader`] of [`AssetEvent<RonAsset<Spec>>`] — asset events
/// are MESSAGES (`bevy-traps.md` #4) — and acts only on a
/// [`Modified`](bevy::asset::AssetEvent::Modified) whose `id` matches the
/// active [`HotRonHandle`] (trap (c)); events for any other handle are ignored,
/// and however many matching events arrive it re-derives at most ONCE per frame
/// from the latest in-memory value. On a match it re-runs the chain's
/// [map hook](HotRonChain::map) (with [`AssetServer`] access, so a hot-edited
/// font key re-loads) and overwrites the resident resource through [`ResMut`]
/// (trap (d)) — marking it CHANGED so change-driven consumers repaint /
/// re-derive the same frame — then `info!`s naming the concrete type + path
/// (the GTW-374 Part C reload-log convention).
///
/// While any needed resource is still absent (pre-resolve) it DRAINS the reader
/// via `events.clear()` (trap (b)) so a stale event never lingers and re-fires
/// once the resources arrive.
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
        // Trap (b): drain the reader so a pre-resolve event does not linger and
        // re-fire once the resources arrive; there is nothing to re-derive yet.
        events.clear();
        return;
    };

    // Trap (c): act only on a Modified for the ACTIVE handle, and at most once
    // per frame — a single re-derive from the latest in-memory value covers
    // every queued event.
    let active_id = handle.id();
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { id } if *id == active_id));
    if !modified {
        return;
    }

    let Some(updated) = assets.get(&**handle) else {
        // Modified but not currently in the collection (a transient reload
        // state) — keep the existing resource until it settles; the next event
        // re-fires.
        return;
    };
    // Trap (d): overwrite THROUGH ResMut so change detection drives downstream
    // repaint / re-derive the same frame.
    *resource = (chain.map())(&**updated, &asset_server);
    // GTW-374 Part C convention: log EVERY hot-reload path naming the concrete
    // type + path, so a live edit can be traced.
    info!(
        "hot-reload: re-derived {} from `{}`",
        short_type_name::<T>(),
        *chain.path(),
    );
}

/// The unqualified name of `T` (`a::b::CombatTuning` -> `CombatTuning`) for the
/// reload / fallback log lines.
fn short_type_name<T>() -> &'static str {
    let full = core::any::type_name::<T>();
    full.rsplit("::").next().unwrap_or(full)
}
