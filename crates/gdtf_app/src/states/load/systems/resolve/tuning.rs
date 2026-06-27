//! GTW-206 (E10.4): resolves the shipped [`CombatTuning`] into the persistent runtime
//! [`CombatTuning`] resource, plus the GTW-374 LIVE hot-reload that re-derives it on a
//! `core_tuning/combat.tuning.ron` edit.

use bevy::{
    asset::{AssetEvent, LoadState},
    prelude::{AssetServer, Assets, Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::tuning::CombatTuning;

use crate::states::load::resources::{ActiveTuningHandle, LoadHandles};

/// GTW-206 (E10.4): resolves the shipped [`CombatTuning`] RON into the persistent
/// runtime [`CombatTuning`] resource, mirroring the theme path's poll/resolve +
/// warn/fallback shape but for a payload that needs NO `resolve()` step
/// (`CombatTuning` is BOTH the `Deserialize` payload AND the `Resource`).
///
/// Called only while no [`CombatTuning`] resource exists yet (the caller's
/// own-absence guard), independently of the theme branch:
///
/// - If the tuning RON reached [`LoadState::Failed`], `warn!`s naming
///   `core_tuning/combat.tuning.ron` and inserts [`CombatTuning::default`] — the ADR-0003
///   sanctioned error-path safety-net — so `Load` always exits with a tuning
///   present and never hangs on a bad tuning file.
/// - Else once the tuning RON is [`LoadState::Loaded`], reads the deserialized
///   [`CombatTuning`] out of `Assets<RonAsset<CombatTuning>>` (the same
///   transient-one-frame `Assets::get` retry the theme path uses) and inserts the
///   inner payload directly as the persistent resource. Like
///   [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) it survives `OnExit(Load)` (it is
///   **not** removed in `cleanup`), because `BattleScape` reads it.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_tuning(
    commands: &mut Commands,
    asset_server: &AssetServer,
    tuning_assets: &Assets<RonAsset<CombatTuning>>,
    handles: &LoadHandles,
) {
    let tuning_state = asset_server.load_state(&*handles.tuning);

    // Failure path: a bad tuning must not hang the app. Warn naming the path and
    // fall back to the const-default tuning so Load always exits with one present.
    if tuning_state.is_failed() {
        warn!(
            "GDTF Load: asset `core_tuning/combat.tuning.ron` failed to load; falling back to the const \
             default combat tuning",
        );
        commands.insert_resource(CombatTuning::default());
        return;
    }

    // Success path: once the tuning RON is loaded, read the deserialized payload
    // out of its collection (transient-one-frame retry like the theme) and insert
    // it directly — CombatTuning is both the payload and the runtime resource.
    if matches!(tuning_state, LoadState::Loaded) {
        let Some(tuning) = tuning_assets.get(&*handles.tuning) else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays
            // alive while CombatTuning is still absent).
            return;
        };
        commands.insert_resource((**tuning).clone());
        // GTW-374: insert the PERSISTENT tuning handle alongside the resource — like
        // the theme's ActiveThemeHandle it survives OnExit(Load), so the live
        // hot-reload handler can filter AssetEvents against it AND re-read the
        // refreshed asset on a `core_tuning/combat.tuning.ron` edit.
        commands.insert_resource(ActiveTuningHandle::new((*handles.tuning).clone()));
    }
}

/// `Update`: re-derive the [`CombatTuning`] resource in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for `core_tuning/combat.tuning.ron`
/// — the GTW-374 LIVE combat-balance hot-reload, modelled on the presenter's
/// `redrive_fx_tuning_on_asset_event` and the UI theme's `redrive_theme_on_asset_event`.
///
/// Reads the [`MessageReader`] of
/// [`AssetEvent`](bevy::asset::AssetEvent)`<`[`RonAsset`]`<`[`CombatTuning`]`>>` — asset
/// events are MESSAGES in Bevy 0.19, so this is a `MessageReader`, not an `EventReader`
/// (`bevy-traps.md` #4) — and acts only on a
/// [`Modified`](bevy::asset::AssetEvent::Modified) event whose `id` matches the
/// PERSISTENT [`ActiveTuningHandle`]; events for any other handle are ignored. On a
/// match it clones the refreshed [`CombatTuning`] out of the `Assets` collection
/// (`CombatTuning` IS both the `Deserialize` payload AND the `Resource`, so no
/// `resolve()` step) and overwrites the resident resource through [`ResMut`], so the
/// sim marches with the new balance coefficients the very next frame — WITHOUT a
/// rebuild. The re-derive carries EVERY tuning leaf through, including a `0`
/// `wound_costs.minor` (a plain `u8`, no `NonZero`).
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it
/// takes the handle / the `Assets` collection / the [`CombatTuning`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1) so a pre-resolve event does not linger and re-fire later.
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional handle /
/// `Assets` / [`CombatTuning`] borrows.
pub(in crate::states::load) fn redrive_combat_tuning_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<CombatTuning>>>,
    handle: Option<Res<ActiveTuningHandle>>,
    tuning_assets: Option<Res<Assets<RonAsset<CombatTuning>>>>,
    tuning: Option<ResMut<CombatTuning>>,
) {
    let (Some(handle), Some(tuning_assets), Some(mut tuning)) = (handle, tuning_assets, tuning)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to re-derive yet.
        events.clear();
        return;
    };

    let active_id = handle.id();
    // Act once per frame even if several Modified events arrive: a single re-derive
    // from the latest in-memory value covers them all.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { id } if *id == active_id));
    if !modified {
        return;
    }

    let Some(updated) = tuning_assets.get(&**handle) else {
        // Modified but not currently in the collection (a transient reload state) —
        // leave the existing tuning until it settles; the next event re-fires.
        return;
    };
    *tuning = (**updated).clone();
    info!("combat hot-reload: re-derived CombatTuning from `core_tuning/combat.tuning.ron`");
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
    use gdtf_battle_sim::tuning::{CombatTuning, WoundCost, WoundCosts};

    use super::redrive_combat_tuning_on_asset_event;
    use crate::states::load::{
        resources::ActiveTuningHandle, systems::resolve::hot_reload_test_support::capture_logs,
    };

    /// A headless app with the real hot-reload wiring: `MinimalPlugins` + `AssetPlugin`
    /// (so `Assets<RonAsset<CombatTuning>>` and the `AssetEvent` message buffer exist),
    /// the `CombatTuning` RON loader registered, and the redrive system in `Update`.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset::<CombatTuning>()
            .add_systems(Update, redrive_combat_tuning_on_asset_event);
        app
    }

    /// Add a `RonAsset<CombatTuning>` to the collection and return its handle.
    fn add_asset(app: &mut App, tuning: CombatTuning) -> Handle<RonAsset<CombatTuning>> {
        app.world_mut()
            .resource_mut::<Assets<RonAsset<CombatTuning>>>()
            .add(RonAsset::new(tuning))
    }

    /// Overwrite the in-memory payload of an already-added tuning asset (the hot edit
    /// the file-watcher would make).
    fn hot_edit(app: &mut App, handle: &Handle<RonAsset<CombatTuning>>, tuning: CombatTuning) {
        let mut assets = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<CombatTuning>>>();
        if let Some(mut asset) = assets.get_mut(handle) {
            **asset = tuning;
        }
    }

    /// Inject an `AssetEvent::Modified` for the given handle id (standing in for the
    /// file-watcher's reload signal).
    fn inject_modified(app: &mut App, handle: &Handle<RonAsset<CombatTuning>>) {
        app.world_mut()
            .write_message(AssetEvent::Modified { id: handle.id() });
    }

    /// Build a `CombatTuning` whose `wound_costs.minor` is `cost` (everything else
    /// default) — the 0-cost preservation fixture.
    fn tuning_with_minor_cost(cost: u8) -> CombatTuning {
        CombatTuning {
            wound_costs: WoundCosts {
                minor: WoundCost::new(cost),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// B1: on a matching `Modified` for the active tuning handle, the redrive system
    /// overwrites the `CombatTuning` resource from the UPDATED in-memory payload — the
    /// live combat-balance hot-reload.
    ///
    /// Pin-discriminating: dropping the re-derive leaves `CombatTuning` on the OLD
    /// `firing_arc`; a wrong id filter would re-derive on any handle.
    #[test]
    fn modified_event_rederives_combat_tuning() {
        let mut app = app();

        let baseline = CombatTuning::default();
        let handle = add_asset(&mut app, baseline.clone());
        app.world_mut().insert_resource(baseline.clone());
        app.world_mut()
            .insert_resource(ActiveTuningHandle::new(handle.clone()));

        // First update: no event, the resource is untouched.
        app.update();

        // Hot-edit the asset to a DISTINCT tuning, then fire a Modified.
        let edited = tuning_with_minor_cost(7);
        assert_ne!(edited, baseline, "precondition: the edit must differ");
        hot_edit(&mut app, &handle, edited.clone());
        inject_modified(&mut app, &handle);
        app.update();

        assert_eq!(
            app.world().get_resource::<CombatTuning>(),
            Some(&edited),
            "a Modified for the active tuning handle must re-derive CombatTuning to the edited value",
        );
    }

    /// B1: a `Modified` for a DIFFERENT asset id leaves `CombatTuning` untouched — the
    /// filter is on the ACTIVE handle id only.
    ///
    /// Pin-discriminating: dropping the id filter re-derives on any tuning asset's event.
    #[test]
    fn modified_event_for_other_id_does_not_rederive() {
        let mut app = app();

        let baseline = CombatTuning::default();
        let active = add_asset(&mut app, baseline.clone());
        // A second, unrelated tuning asset whose value differs from the active one.
        let other = add_asset(&mut app, tuning_with_minor_cost(9));
        app.world_mut().insert_resource(baseline.clone());
        app.world_mut()
            .insert_resource(ActiveTuningHandle::new(active));

        app.update();
        // Edit the OTHER asset, fire Modified for it only.
        hot_edit(&mut app, &other, tuning_with_minor_cost(3));
        inject_modified(&mut app, &other);
        app.update();

        assert_eq!(
            app.world().get_resource::<CombatTuning>(),
            Some(&baseline),
            "a Modified for a non-active tuning id must NOT re-derive CombatTuning",
        );
    }

    /// 0-COST WOUND: a `wound_costs.minor` of `0` survives BOTH the initial load
    /// (the resolve inserts the deserialized payload verbatim) AND a hot-reload (the
    /// redrive carries it through) — `WoundCost` is a plain `u8`, no `NonZero`, so `0`
    /// is a valid, preserved authored cost.
    ///
    /// Pin-discriminating: if the re-derive ever coerced `0` to a non-zero (e.g. a
    /// `NonZero` newtype, or a `max(1, …)`), the post-reload assert fails.
    #[test]
    fn zero_minor_wound_cost_survives_load_and_hot_reload() {
        let mut app = app();

        // Load-time: the resident resource already carries a 0 minor cost (standing in
        // for the resolve having inserted the deserialized payload).
        let loaded = tuning_with_minor_cost(0);
        let handle = add_asset(&mut app, loaded.clone());
        app.world_mut().insert_resource(loaded);
        app.world_mut()
            .insert_resource(ActiveTuningHandle::new(handle.clone()));
        app.update();
        assert_eq!(
            app.world()
                .get_resource::<CombatTuning>()
                .map(|t| *t.wound_costs.minor),
            Some(0),
            "a 0 minor wound cost must survive the initial load verbatim",
        );

        // Hot-reload: edit the asset (still a 0 minor cost, but a distinct sibling field
        // so the value genuinely changes), fire Modified, and confirm the 0 carries
        // through the re-derive.
        let edited = CombatTuning {
            wound_costs: WoundCosts {
                minor:    WoundCost::new(0),
                major:    WoundCost::new(5),
                critical: WoundCost::new(9),
            },
            ..Default::default()
        };
        hot_edit(&mut app, &handle, edited.clone());
        inject_modified(&mut app, &handle);
        app.update();
        assert_eq!(
            app.world().get_resource::<CombatTuning>(),
            Some(&edited),
            "the hot-reload must re-derive to the edited tuning",
        );
        assert_eq!(
            app.world()
                .get_resource::<CombatTuning>()
                .map(|t| *t.wound_costs.minor),
            Some(0),
            "a 0 minor wound cost must survive the hot-reload (WoundCost is a plain u8, no NonZero)",
        );
    }

    /// Part C: the hot-reload `info!` line FIRES on the real re-derive path, naming
    /// what reloaded. The redrive system is run via `run_system_once` on the calling
    /// thread (inside the scoped `tracing` subscriber) so the capture — which is
    /// thread-local — sees the emission; the schedule executor may run systems on a
    /// worker thread, which the capture would miss.
    ///
    /// Pin-discriminating: removing the `info!` from the redrive leaves the capture
    /// empty and this assert fails.
    #[test]
    fn hot_reload_logs_an_info_line() {
        let mut app = app();
        let baseline = CombatTuning::default();
        let handle = add_asset(&mut app, baseline.clone());
        app.world_mut().insert_resource(baseline);
        app.world_mut()
            .insert_resource(ActiveTuningHandle::new(handle.clone()));
        // Stage the hot edit + the Modified message, then run the redrive synchronously
        // inside the capture scope.
        hot_edit(&mut app, &handle, tuning_with_minor_cost(7));
        inject_modified(&mut app, &handle);

        let captured = capture_logs(|| {
            let result = app
                .world_mut()
                .run_system_once(redrive_combat_tuning_on_asset_event);
            assert!(result.is_ok(), "the redrive system must run cleanly");
        });

        assert!(
            captured
                .iter()
                .any(|line| line.contains("combat hot-reload") && line.contains("CombatTuning")),
            "the combat-tuning hot-reload must emit an info! line naming what reloaded; \
             captured: {captured:?}",
        );
    }
}
