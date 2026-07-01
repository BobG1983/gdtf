//! GTW-533: resolves the shipped [`ProcgenTuning`] into the persistent runtime
//! [`ProcgenTuning`] resource, plus the LIVE hot-reload that re-reads it on a
//! `core_tuning/procgen.tuning.ron` edit.
//!
//! Before GTW-533 the procgen fill pass's knobs shipped as an asset
//! (`assets/core_tuning/procgen.tuning.ron`) that was DOCUMENTED as hot-reloadable but was
//! never asset-loaded: every consumer built `ProcgenTuning::default()` (a hardcoded RULED
//! default), so an edit to the shipped file did NOTHING. This resolve wires the file the
//! same way GTW-206 wired [`CombatTuning`](gdtf_battle_sim::tuning::CombatTuning) — the
//! payload IS the runtime resource — so the procgen fill density / large-prefab threshold /
//! dead-rect scatter cap now tune WITHOUT a rebuild, closing the audit gap.

use bevy::{
    asset::{AssetEvent, LoadState},
    prelude::{AssetServer, Assets, Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::procgen::ProcgenTuning;

use crate::states::load::resources::{ActiveProcgenTuningHandle, LoadHandles};

/// GTW-533: resolves the shipped [`ProcgenTuning`] RON into the persistent runtime
/// [`ProcgenTuning`] resource, mirroring the combat-tuning
/// [`resolve_tuning`](super::tuning::resolve_tuning) poll/resolve + warn/fallback shape
/// (the payload needs NO `resolve()` step — `ProcgenTuning` is BOTH the `Deserialize`
/// payload AND the `Resource`).
///
/// Called only while no [`ProcgenTuning`] resource exists yet (the caller's own-absence
/// guard), independently of the other branches:
///
/// - If the procgen-tuning RON reached [`LoadState::Failed`], `warn!`s naming
///   `core_tuning/procgen.tuning.ron` and inserts [`ProcgenTuning::default`] — the ADR-0003
///   sanctioned error-path safety-net (the SAME RULED default the pass used before GTW-533
///   wired the load) — so `Load` always exits with a procgen tuning present and never hangs
///   on a bad file.
/// - Else once the procgen-tuning RON is [`LoadState::Loaded`], reads the deserialized
///   [`ProcgenTuning`] out of `Assets<RonAsset<ProcgenTuning>>` (the same
///   transient-one-frame `Assets::get` retry the theme / combat-tuning paths use) and
///   inserts the inner payload directly as the persistent resource. Like
///   [`CombatTuning`](gdtf_battle_sim::tuning::CombatTuning) it survives `OnExit(Load)` (it
///   is **not** removed in `cleanup`), because the Generation procgen trigger reads it.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_procgen_tuning(
    commands: &mut Commands,
    asset_server: &AssetServer,
    procgen_assets: &Assets<RonAsset<ProcgenTuning>>,
    handles: &LoadHandles,
) {
    let procgen_state = asset_server.load_state(&*handles.procgen);

    // Failure path: a bad procgen tuning must not hang the app. Warn naming the path and
    // fall back to the const-default tuning so Load always exits with one present.
    if procgen_state.is_failed() {
        warn!(
            "GDTF Load: asset `core_tuning/procgen.tuning.ron` failed to load; falling back to the \
             const default procgen tuning",
        );
        commands.insert_resource(ProcgenTuning::default());
        return;
    }

    // Success path: once the procgen tuning RON is loaded, read the deserialized payload
    // out of its collection (transient-one-frame retry like the theme) and insert it
    // directly — ProcgenTuning is both the payload and the runtime resource.
    if matches!(procgen_state, LoadState::Loaded) {
        let Some(procgen) = procgen_assets.get(&*handles.procgen) else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays alive
            // while ProcgenTuning is still absent).
            return;
        };
        commands.insert_resource(**procgen);
        // GTW-533: insert the PERSISTENT procgen-tuning handle alongside the resource — like
        // the combat tuning's ActiveTuningHandle it survives OnExit(Load), so the live
        // hot-reload handler can filter AssetEvents against it AND re-read the refreshed
        // asset on a `core_tuning/procgen.tuning.ron` edit.
        commands.insert_resource(ActiveProcgenTuningHandle::new((*handles.procgen).clone()));
    }
}

/// `Update`: overwrite the resident [`ProcgenTuning`] resource in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for
/// `core_tuning/procgen.tuning.ron` — the GTW-533 LIVE procgen-tuning hot-reload, modelled
/// on the combat-tuning redrive
/// ([`redrive_combat_tuning_on_asset_event`](super::tuning::redrive_combat_tuning_on_asset_event)).
///
/// Reads the [`MessageReader`] of
/// [`AssetEvent`](bevy::asset::AssetEvent)`<`[`RonAsset`]`<`[`ProcgenTuning`]`>>` — asset
/// events are MESSAGES in Bevy 0.19, so this is a [`MessageReader`], not an `EventReader`
/// (`bevy-traps.md` #4) — and acts only on a
/// [`Modified`](bevy::asset::AssetEvent::Modified) event whose `id` matches the PERSISTENT
/// [`ActiveProcgenTuningHandle`]; events for any other handle are ignored. On a match it
/// clones the refreshed [`ProcgenTuning`] out of the `Assets` collection (`ProcgenTuning`
/// IS both the `Deserialize` payload AND the `Resource`, so no `resolve()` step) and
/// overwrites the resident resource through [`ResMut`], so the NEXT battle generation packs
/// with the new fill knobs — WITHOUT a rebuild.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes the
/// handle / the `Assets` collection / the [`ProcgenTuning`] resource as [`Option`]al
/// borrows, draining the reader and returning early if any is missing (`bevy-traps.md` #1)
/// so a pre-resolve event does not linger and re-fire later.
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional handle / `Assets` /
/// [`ProcgenTuning`] borrows.
pub(in crate::states::load) fn redrive_procgen_tuning_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<ProcgenTuning>>>,
    handle: Option<Res<ActiveProcgenTuningHandle>>,
    procgen_assets: Option<Res<Assets<RonAsset<ProcgenTuning>>>>,
    procgen: Option<ResMut<ProcgenTuning>>,
) {
    let (Some(handle), Some(procgen_assets), Some(mut procgen)) = (handle, procgen_assets, procgen)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to re-read yet.
        events.clear();
        return;
    };

    let active_id = handle.id();
    // Act once per frame even if several Modified events arrive: a single overwrite from
    // the latest in-memory value covers them all.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { id } if *id == active_id));
    if !modified {
        return;
    }

    let Some(updated) = procgen_assets.get(&**handle) else {
        // Modified but not currently in the collection (a transient reload state) — leave
        // the existing tuning until it settles; the next event re-fires.
        return;
    };
    *procgen = **updated;
    // GTW-374 Part C convention: log EVERY hot-reload path naming what reloaded.
    info!("procgen hot-reload: reloaded ProcgenTuning from `core_tuning/procgen.tuning.ron`");
}

#[cfg(test)]
mod test {
    use bevy::{
        MinimalPlugins,
        asset::{AssetEvent, AssetPlugin, Assets, Handle},
        prelude::*,
    };
    use gdtf_assets::{RonAsset, RonAssetAppExt};
    use gdtf_battle_sim::procgen::{DeadRectScatterCount, ProcgenTuning};

    use super::redrive_procgen_tuning_on_asset_event;
    use crate::states::load::resources::ActiveProcgenTuningHandle;

    /// A headless app with the REAL procgen-tuning hot-reload wiring: `MinimalPlugins` +
    /// `AssetPlugin` (so `Assets<RonAsset<ProcgenTuning>>` and the `AssetEvent` message
    /// buffer exist), the `ProcgenTuning` RON loader registered, and the redrive system in
    /// `Update` — the same shape the combat-tuning hot-reload test uses. This drives the
    /// ACTUAL production redrive system (`redrive_procgen_tuning_on_asset_event`, wired into
    /// the game's `add_hot_reload_systems`), not a copy.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset::<ProcgenTuning>()
            .add_systems(Update, redrive_procgen_tuning_on_asset_event);
        app
    }

    /// Add a `RonAsset<ProcgenTuning>` to the collection and return its handle.
    fn add_asset(app: &mut App, tuning: ProcgenTuning) -> Handle<RonAsset<ProcgenTuning>> {
        app.world_mut()
            .resource_mut::<Assets<RonAsset<ProcgenTuning>>>()
            .add(RonAsset::new(tuning))
    }

    /// Overwrite the in-memory payload of an already-added tuning asset — exactly what the
    /// real file-watcher does when the loose `.ron` on disk is re-read.
    fn hot_edit(app: &mut App, handle: &Handle<RonAsset<ProcgenTuning>>, tuning: ProcgenTuning) {
        let mut assets = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<ProcgenTuning>>>();
        if let Some(mut asset) = assets.get_mut(handle) {
            **asset = tuning;
        }
    }

    /// Inject an `AssetEvent::Modified` for the given handle id — the signal the real
    /// file-watcher emits when it re-reads the changed file.
    fn inject_modified(app: &mut App, handle: &Handle<RonAsset<ProcgenTuning>>) {
        app.world_mut()
            .write_message(AssetEvent::Modified { id: handle.id() });
    }

    /// Build a `ProcgenTuning` whose `dead_rect_scatter_count_k` is `k` (everything else
    /// default) — a distinguishing, comparable field (the newtype derives `PartialEq`).
    fn tuning_with_scatter_k(k: u8) -> ProcgenTuning {
        ProcgenTuning {
            dead_rect_scatter_count_k: DeadRectScatterCount::new(k),
            ..Default::default()
        }
    }

    /// C3 (GTW-533): a previously-non-reloading asset — the shipped [`ProcgenTuning`] — now
    /// hot-reloads. On a matching `Modified` for the active handle, the redrive overwrites
    /// the resident [`ProcgenTuning`] from the UPDATED in-memory payload WITHOUT any app
    /// restart — the real watcher+reload path, standing in the file-watcher with an
    /// in-memory edit + a `Modified` message.
    ///
    /// Pin-discriminating: dropping the redrive leaves `ProcgenTuning` on the OLD scatter
    /// cap; a wrong id filter would re-read on any handle.
    #[test]
    fn modified_event_reloads_procgen_tuning() {
        let mut app = app();

        let baseline = tuning_with_scatter_k(3);
        let handle = add_asset(&mut app, baseline);
        app.world_mut().insert_resource(baseline);
        app.world_mut()
            .insert_resource(ActiveProcgenTuningHandle::new(handle.clone()));

        // First update: no event, the resource is untouched.
        app.update();

        // Hot-edit the asset to a DISTINCT tuning, then fire a Modified.
        let edited = tuning_with_scatter_k(7);
        assert_ne!(edited, baseline, "precondition: the edit must differ");
        hot_edit(&mut app, &handle, edited);
        inject_modified(&mut app, &handle);
        app.update();

        assert_eq!(
            app.world().get_resource::<ProcgenTuning>(),
            Some(&edited),
            "a Modified for the active procgen-tuning handle must reload ProcgenTuning to the \
             edited value WITHOUT a restart",
        );
    }

    /// C3: a `Modified` for a DIFFERENT asset id leaves `ProcgenTuning` untouched — the
    /// filter is on the ACTIVE handle id only.
    ///
    /// Pin-discriminating: dropping the id filter re-reads on any procgen-tuning asset's
    /// event.
    #[test]
    fn modified_event_for_other_id_does_not_reload() {
        let mut app = app();

        let baseline = tuning_with_scatter_k(3);
        let active = add_asset(&mut app, baseline);
        // A second, unrelated procgen-tuning asset whose value differs from the active one.
        let other = add_asset(&mut app, tuning_with_scatter_k(9));
        app.world_mut().insert_resource(baseline);
        app.world_mut()
            .insert_resource(ActiveProcgenTuningHandle::new(active));

        app.update();
        // Edit the OTHER asset, fire Modified for it only.
        hot_edit(&mut app, &other, tuning_with_scatter_k(5));
        inject_modified(&mut app, &other);
        app.update();

        assert_eq!(
            app.world().get_resource::<ProcgenTuning>(),
            Some(&baseline),
            "a Modified for a non-active procgen-tuning id must NOT reload ProcgenTuning",
        );
    }
}
