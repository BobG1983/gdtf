//! GTW-384: resolves the shipped [`GangerStatTuning`] into the persistent runtime
//! [`GangerStatTuning`] resource, plus the LIVE hot-reload that re-derives it on a
//! `combat/stat_tuning.ron` edit — the GTW-374 combat-tuning loader + hot-reload mirror.

use bevy::{
    asset::{AssetEvent, LoadState},
    prelude::{AssetServer, Assets, Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::tuning::GangerStatTuning;

use crate::states::load::resources::{ActiveStatTuningHandle, LoadHandles};

/// GTW-384: resolves the shipped [`GangerStatTuning`] RON into the persistent runtime
/// [`GangerStatTuning`] resource, MIRRORING [`resolve_tuning`](super::tuning::resolve_tuning)
/// (the `CombatTuning` path) exactly — a payload that IS both the `Deserialize` payload
/// AND the `Resource`, so no `resolve()` step.
///
/// Called only while no [`GangerStatTuning`] resource exists yet (the caller's
/// own-absence guard), independently of the other resolve branches:
///
/// - If the stat-tuning RON reached [`LoadState::Failed`], `warn!`s naming
///   `combat/stat_tuning.ron` and inserts [`GangerStatTuning::default`] — the
///   ADR-0003 error-path safety-net — so `Load` always exits with a stat tuning present
///   and never hangs on a bad file. The default carries the stats.md flat-`1.0` weights
///   (Cool `0.5` into HP) + `~10` divisors, so the derivation still produces sensible
///   stats.
/// - Else once the RON is [`LoadState::Loaded`], reads the deserialized
///   [`GangerStatTuning`] out of `Assets<RonAsset<GangerStatTuning>>` and inserts the
///   inner payload directly as the persistent resource. Like `CombatTuning` it survives
///   `OnExit(Load)` (it is **not** removed in `cleanup`), because the sim's setup +
///   re-derive read it throughout the battle.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_stat_tuning(
    commands: &mut Commands,
    asset_server: &AssetServer,
    stat_tuning_assets: &Assets<RonAsset<GangerStatTuning>>,
    handles: &LoadHandles,
) {
    let state = asset_server.load_state(&*handles.stat_tuning);

    // Failure path: a bad stat tuning must not hang the app. Warn naming the path and
    // fall back to the const-default tuning so Load always exits with one present.
    if state.is_failed() {
        warn!(
            "GDTF Load: asset `combat/stat_tuning.ron` failed to load; falling back to the const \
             default ganger stat tuning",
        );
        commands.insert_resource(GangerStatTuning::default());
        return;
    }

    // Success path: once the RON is loaded, read the deserialized payload out of its
    // collection (transient-one-frame retry like the theme) and insert it directly —
    // GangerStatTuning is both the payload and the runtime resource.
    if matches!(state, LoadState::Loaded) {
        let Some(tuning) = stat_tuning_assets.get(&*handles.stat_tuning) else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays alive
            // while GangerStatTuning is still absent).
            return;
        };
        commands.insert_resource((**tuning).clone());
        // Insert the PERSISTENT handle alongside the resource — like ActiveTuningHandle it
        // survives OnExit(Load), so the live hot-reload handler can filter AssetEvents
        // against it AND re-read the refreshed asset on a `combat/stat_tuning.ron` edit.
        commands.insert_resource(ActiveStatTuningHandle::new((*handles.stat_tuning).clone()));
    }
}

/// `Update`: re-derive the [`GangerStatTuning`] resource in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for
/// `combat/stat_tuning.ron` — the GTW-384 LIVE stat-derivation hot-reload, modelled on
/// `redrive_combat_tuning_on_asset_event` (GTW-374).
///
/// Reads the [`MessageReader`] of
/// [`AssetEvent`](bevy::asset::AssetEvent)`<`[`RonAsset`]`<`[`GangerStatTuning`]`>>` —
/// asset events are MESSAGES in Bevy 0.19 (`bevy-traps.md` #4) — and acts only on a
/// [`Modified`](bevy::asset::AssetEvent::Modified) event whose `id` matches the PERSISTENT
/// [`ActiveStatTuningHandle`]; events for any other handle are ignored. On a match it
/// clones the refreshed [`GangerStatTuning`] out of the `Assets` collection and overwrites
/// the resident resource through [`ResMut`]. The resulting `Changed<GangerStatTuning>`
/// trips the sim's `rederive_stats_on_tuning_change` (GTW-384), so EVERY spawned ganger's
/// computed stats re-derive and its current pools clamp to the new maxes the very next
/// frame — WITHOUT a rebuild.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes
/// the handle / the `Assets` collection / the [`GangerStatTuning`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1) so a pre-resolve event does not linger and re-fire later.
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional handle /
/// `Assets` / [`GangerStatTuning`] borrows.
pub(in crate::states::load) fn redrive_stat_tuning_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<GangerStatTuning>>>,
    handle: Option<Res<ActiveStatTuningHandle>>,
    stat_tuning_assets: Option<Res<Assets<RonAsset<GangerStatTuning>>>>,
    tuning: Option<ResMut<GangerStatTuning>>,
) {
    let (Some(handle), Some(stat_tuning_assets), Some(mut tuning)) =
        (handle, stat_tuning_assets, tuning)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to re-derive yet.
        events.clear();
        return;
    };

    let active_id = handle.id();
    // Act once per frame even if several Modified events arrive: a single re-derive from
    // the latest in-memory value covers them all.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { id } if *id == active_id));
    if !modified {
        return;
    }

    let Some(updated) = stat_tuning_assets.get(&**handle) else {
        // Modified but not currently in the collection (a transient reload state) — leave
        // the existing tuning until it settles; the next event re-fires.
        return;
    };
    *tuning = (**updated).clone();
    info!("combat hot-reload: re-derived GangerStatTuning from `combat/stat_tuning.ron`");
}

#[cfg(test)]
mod test {
    use bevy::{
        MinimalPlugins,
        asset::{AssetEvent, AssetPlugin, Assets, Handle},
        ecs::system::RunSystemOnce,
        prelude::*,
    };
    use gdtf_assets::{RonAsset, RonAssetAppExt};
    use gdtf_battle_sim::tuning::{GangerStatTuning, StatWeight};

    use super::redrive_stat_tuning_on_asset_event;
    use crate::states::load::{
        resources::ActiveStatTuningHandle, systems::resolve::hot_reload_test_support::capture_logs,
    };

    /// A headless app with the real hot-reload wiring: `MinimalPlugins` + `AssetPlugin`
    /// (so `Assets<RonAsset<GangerStatTuning>>` and the `AssetEvent` message buffer
    /// exist), the loader registered, and the redrive system in `Update`.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset::<GangerStatTuning>()
            .add_systems(Update, redrive_stat_tuning_on_asset_event);
        app
    }

    /// Add a `RonAsset<GangerStatTuning>` to the collection and return its handle.
    fn add_asset(app: &mut App, tuning: GangerStatTuning) -> Handle<RonAsset<GangerStatTuning>> {
        app.world_mut()
            .resource_mut::<Assets<RonAsset<GangerStatTuning>>>()
            .add(RonAsset(tuning))
    }

    /// Overwrite the in-memory payload of an already-added stat-tuning asset (the hot edit
    /// the file-watcher would make).
    fn hot_edit(
        app: &mut App,
        handle: &Handle<RonAsset<GangerStatTuning>>,
        tuning: GangerStatTuning,
    ) {
        let mut assets = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<GangerStatTuning>>>();
        if let Some(mut asset) = assets.get_mut(handle) {
            asset.0 = tuning;
        }
    }

    /// Inject an `AssetEvent::Modified` for the given handle id (standing in for the
    /// file-watcher's reload signal).
    fn inject_modified(app: &mut App, handle: &Handle<RonAsset<GangerStatTuning>>) {
        app.world_mut()
            .write_message(AssetEvent::Modified { id: handle.id() });
    }

    /// Build a `GangerStatTuning` whose Shooting `aim` weight is `weight` (everything else
    /// default) — a distinct edit fixture.
    fn tuning_with_aim_weight(weight: f32) -> GangerStatTuning {
        let mut t = GangerStatTuning::default();
        t.shooting.aim = StatWeight::new(weight);
        t
    }

    /// On a matching `Modified` for the active stat-tuning handle, the redrive system
    /// overwrites the `GangerStatTuning` resource from the UPDATED in-memory payload — the
    /// live stat-derivation hot-reload. Pin-discriminating: dropping the re-derive leaves
    /// the OLD weights; a wrong id filter would re-derive on any handle.
    #[test]
    fn modified_event_rederives_stat_tuning() {
        let mut app = app();

        let baseline = GangerStatTuning::default();
        let handle = add_asset(&mut app, baseline.clone());
        app.world_mut().insert_resource(baseline.clone());
        app.world_mut()
            .insert_resource(ActiveStatTuningHandle::new(handle.clone()));

        app.update();

        let edited = tuning_with_aim_weight(7.0);
        assert_ne!(edited, baseline, "precondition: the edit must differ");
        hot_edit(&mut app, &handle, edited.clone());
        inject_modified(&mut app, &handle);
        app.update();

        assert_eq!(
            app.world().get_resource::<GangerStatTuning>(),
            Some(&edited),
            "a Modified for the active stat-tuning handle must re-derive GangerStatTuning",
        );
    }

    /// A `Modified` for a DIFFERENT asset id leaves `GangerStatTuning` untouched — the
    /// filter is on the ACTIVE handle id only. Pin-discriminating: dropping the id filter
    /// re-derives on any stat-tuning asset's event.
    #[test]
    fn modified_event_for_other_id_does_not_rederive() {
        let mut app = app();

        let baseline = GangerStatTuning::default();
        let active = add_asset(&mut app, baseline.clone());
        let other = add_asset(&mut app, tuning_with_aim_weight(9.0));
        app.world_mut().insert_resource(baseline.clone());
        app.world_mut()
            .insert_resource(ActiveStatTuningHandle::new(active));

        app.update();
        hot_edit(&mut app, &other, tuning_with_aim_weight(3.0));
        inject_modified(&mut app, &other);
        app.update();

        assert_eq!(
            app.world().get_resource::<GangerStatTuning>(),
            Some(&baseline),
            "a Modified for a non-active stat-tuning id must NOT re-derive GangerStatTuning",
        );
    }

    /// The hot-reload `info!` line FIRES on the real re-derive path, naming what reloaded.
    /// Run via `run_system_once` on the calling thread (inside the scoped `tracing`
    /// subscriber) so the thread-local capture sees the emission. Pin-discriminating:
    /// removing the `info!` leaves the capture empty.
    #[test]
    fn hot_reload_logs_an_info_line() {
        let mut app = app();
        let baseline = GangerStatTuning::default();
        let handle = add_asset(&mut app, baseline.clone());
        app.world_mut().insert_resource(baseline);
        app.world_mut()
            .insert_resource(ActiveStatTuningHandle::new(handle.clone()));
        hot_edit(&mut app, &handle, tuning_with_aim_weight(7.0));
        inject_modified(&mut app, &handle);

        let captured = capture_logs(|| {
            let result = app
                .world_mut()
                .run_system_once(redrive_stat_tuning_on_asset_event);
            assert!(result.is_ok(), "the redrive system must run cleanly");
        });

        assert!(
            captured.iter().any(|line| {
                line.contains("combat hot-reload") && line.contains("GangerStatTuning")
            }),
            "the stat-tuning hot-reload must emit an info! line naming what reloaded; \
             captured: {captured:?}",
        );
    }
}
