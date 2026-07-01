//! GTW-205 / GTW-261: resolves the authored [`Situation`] into the persistent,
//! gate-blocking [`LoadedSituation`] resource.

use bevy::{
    asset::{AssetEvent, LoadState},
    prelude::{AssetServer, Assets, Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::situation::Situation;

use crate::states::load::resources::{ActiveSituationHandle, LoadHandles, LoadedSituation};

/// GTW-205 / GTW-261: resolves the authored [`Situation`] RON into the persistent
/// [`LoadedSituation`] resource, mirroring the [`resolve_tuning`](super::resolve_tuning)
/// poll/resolve + warn/fallback shape.
///
/// As of GTW-261 the situation is a **gate-blocking** resource (the empty-battle-race
/// fix): the Load→Intro transition now requires a `LoadedSituation` (see the plugin
/// wiring), so a battle never starts before its real situation loads. This resolve
/// makes the situation exactly symmetric with the tuning and weapons branches — it
/// gates entry AND falls back to an empty default on failure, so a slow/failed
/// situation still always lets `Load` exit (the no-strand guarantee GTW-205 wanted,
/// preserved via the failure fallback rather than via non-blocking resolution).
///
/// Called only while no [`LoadedSituation`] resource exists yet (the caller's
/// own-absence guard), independently of the theme / tuning / weapons branches:
///
/// - If the situation RON reached [`LoadState::Failed`], `warn!`s naming
///   `situations/skirmish.ron` and inserts an empty [`Situation::default`] — the
///   ADR-0003 sanctioned error-path safety-net — so `Load` always exits with a
///   situation present and never hangs on a bad situation file. CRITICAL: the empty
///   default is inserted ONLY on a genuine `Failed`, NEVER while the situation is
///   still loading — inserting it early would clear the gate before the real
///   battlefield resolves, re-introducing the empty-battle bug.
/// - Else once the situation RON is [`LoadState::Loaded`], reads the deserialized
///   [`Situation`] out of `Assets<RonAsset<Situation>>` (the same transient-one-frame
///   `Assets::get` retry the theme path uses) and inserts the inner payload as the
///   persistent [`LoadedSituation`]. Like
///   [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) it survives `OnExit(Load)` (it is
///   **not** removed in `cleanup`), because the Generation slice (E10.5) reads it.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_situation(
    commands: &mut Commands,
    asset_server: &AssetServer,
    situation_assets: &Assets<RonAsset<Situation>>,
    handles: &LoadHandles,
) {
    let situation_state = asset_server.load_state(&*handles.situation);

    // Failure path: a bad/missing situation must not hang the app. Warn naming the
    // path and fall back to the empty default situation so Load always exits with one
    // present (a Failed → empty default; the battle then has zero gangers rather than
    // stranding the machine). Only on a genuine Failed — never while still loading.
    if situation_state.is_failed() {
        warn!(
            "GDTF Load: asset `situations/skirmish.ron` failed to load; falling back to the empty \
             default situation (the battle will have no gangers)",
        );
        commands.insert_resource(LoadedSituation::new(Situation::default()));
        return;
    }

    // Success path: once the situation RON is loaded, read the deserialized payload
    // out of its collection (transient-one-frame retry like the theme) and insert it
    // as the persistent LoadedSituation.
    if matches!(situation_state, LoadState::Loaded) {
        let Some(situation) = situation_assets.get(&*handles.situation) else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays
            // alive while LoadedSituation is still absent).
            return;
        };
        // Persist the resolved battlefield for the Generation consumer (E10.5);
        // like GdtfTheme it survives OnExit(Load) (not removed in cleanup).
        commands.insert_resource(LoadedSituation::new((**situation).clone()));
        // GTW-533: insert the PERSISTENT situation handle alongside the resource — like
        // the tuning's ActiveTuningHandle it survives OnExit(Load), so the live
        // hot-reload handler can filter AssetEvents against it AND re-read the refreshed
        // asset on a `situations/skirmish.ron` edit.
        commands.insert_resource(ActiveSituationHandle::new((*handles.situation).clone()));
    }
}

/// `Update` (ungated): overwrite the resident [`LoadedSituation`] resource in place on a
/// matching [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for
/// `situations/skirmish.ron` — the GTW-533 LIVE situation hot-reload, modelled on the
/// combat-tuning redrive
/// ([`redrive_combat_tuning_on_asset_event`](super::tuning::redrive_combat_tuning_on_asset_event)).
///
/// Reads the [`MessageReader`] of
/// [`AssetEvent`](bevy::asset::AssetEvent)`<`[`RonAsset`]`<`[`Situation`]`>>` — asset
/// events are MESSAGES in Bevy 0.19, so this is a [`MessageReader`], not an
/// `EventReader` (`bevy-traps.md` #4) — and acts only on a
/// [`Modified`](bevy::asset::AssetEvent::Modified) event whose `id` matches the
/// persistent [`ActiveSituationHandle`]; events for any other handle are ignored. On a
/// match it clones the refreshed [`Situation`] out of the `Assets` collection and
/// overwrites the resident [`LoadedSituation`] through [`ResMut`], so the next battle
/// GENERATION reads the edited battlefield — WITHOUT a rebuild or restart.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes
/// the handle / the `Assets` collection / the [`LoadedSituation`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1) so a pre-resolve event does not linger and re-fire later.
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional handle /
/// `Assets` / [`LoadedSituation`] borrows.
pub(in crate::states::load) fn redrive_situation_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<Situation>>>,
    handle: Option<Res<ActiveSituationHandle>>,
    situation_assets: Option<Res<Assets<RonAsset<Situation>>>>,
    situation: Option<ResMut<LoadedSituation>>,
) {
    let (Some(handle), Some(situation_assets), Some(mut situation)) =
        (handle, situation_assets, situation)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to re-resolve yet.
        events.clear();
        return;
    };

    let active_id = handle.id();
    // Act once per frame even if several Modified events arrive: a single overwrite
    // from the latest in-memory value covers them all.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { id } if *id == active_id));
    if !modified {
        return;
    }

    let Some(updated) = situation_assets.get(&**handle) else {
        // Modified but not currently in the collection (a transient reload state) —
        // leave the existing situation until it settles; the next event re-fires.
        return;
    };
    *situation = LoadedSituation::new((**updated).clone());
    // GTW-374 Part C convention: log EVERY hot-reload path naming what reloaded.
    info!("situation hot-reload: reloaded LoadedSituation from `situations/skirmish.ron`");
}

#[cfg(test)]
mod test {
    use bevy::{
        MinimalPlugins,
        asset::{AssetEvent, AssetPlugin, Assets, Handle},
        prelude::*,
    };
    use gdtf_assets::{RonAsset, RonAssetAppExt};
    use gdtf_battle_sim::{Faction, situation::Situation};

    use super::redrive_situation_on_asset_event;
    use crate::states::load::resources::{ActiveSituationHandle, LoadedSituation};

    /// A headless app with the REAL situation hot-reload wiring: `MinimalPlugins` +
    /// `AssetPlugin` (so `Assets<RonAsset<Situation>>` and the `AssetEvent` message buffer
    /// exist), the `Situation` RON loader registered, and the redrive system in `Update` —
    /// the same shape the combat-tuning hot-reload test uses. This drives the ACTUAL
    /// production redrive system (`redrive_situation_on_asset_event`, wired into the game's
    /// `add_hot_reload_systems`), not a copy.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset::<Situation>()
            .add_systems(Update, redrive_situation_on_asset_event);
        app
    }

    /// Add a `RonAsset<Situation>` to the collection and return its handle.
    fn add_asset(app: &mut App, situation: Situation) -> Handle<RonAsset<Situation>> {
        app.world_mut()
            .resource_mut::<Assets<RonAsset<Situation>>>()
            .add(RonAsset::new(situation))
    }

    /// Overwrite the in-memory payload of an already-added situation asset — exactly what
    /// the real file-watcher does when the loose `.ron` on disk is re-read.
    fn hot_edit(app: &mut App, handle: &Handle<RonAsset<Situation>>, situation: Situation) {
        let mut assets = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<Situation>>>();
        if let Some(mut asset) = assets.get_mut(handle) {
            **asset = situation;
        }
    }

    /// Inject an `AssetEvent::Modified` for the given handle id — the signal the real
    /// file-watcher emits when it re-reads the changed file.
    fn inject_modified(app: &mut App, handle: &Handle<RonAsset<Situation>>) {
        app.world_mut()
            .write_message(AssetEvent::Modified { id: handle.id() });
    }

    /// Build a `Situation` whose `player_faction` is `gang` (everything else default) — a
    /// distinguishing, comparable field (`Faction` derives `PartialEq`, `Situation` does
    /// not).
    fn situation_with_player_faction(gang: u8) -> Situation {
        Situation {
            player_faction: Faction::new(gang),
            ..Default::default()
        }
    }

    /// C3 (GTW-533): a previously-non-reloading asset — the authored [`Situation`] — now
    /// hot-reloads. On a matching `Modified` for the active situation handle, the redrive
    /// overwrites the resident [`LoadedSituation`] from the UPDATED in-memory payload
    /// WITHOUT any app restart — the real watcher+reload path, standing in the file-watcher
    /// with an in-memory edit + a `Modified` message.
    ///
    /// Pin-discriminating: dropping the redrive leaves `LoadedSituation` on the OLD faction;
    /// a wrong id filter would re-resolve on any handle.
    #[test]
    fn modified_event_reloads_situation() {
        let mut app = app();

        let baseline = situation_with_player_faction(0);
        let handle = add_asset(&mut app, baseline.clone());
        app.world_mut()
            .insert_resource(LoadedSituation::new(baseline.clone()));
        app.world_mut()
            .insert_resource(ActiveSituationHandle::new(handle.clone()));

        // First update: no event, the resource is untouched.
        app.update();

        // Hot-edit the asset to a DISTINCT battlefield, then fire a Modified.
        let edited = situation_with_player_faction(3);
        hot_edit(&mut app, &handle, edited.clone());
        inject_modified(&mut app, &handle);
        app.update();

        // Precondition sanity: the edit differs from the baseline.
        assert_ne!(
            *edited.player_faction, *baseline.player_faction,
            "precondition: the edit must differ from the baseline",
        );
        let reloaded = app
            .world()
            .get_resource::<LoadedSituation>()
            .map(|s| *s.player_faction);
        assert_eq!(
            reloaded,
            Some(*edited.player_faction),
            "a Modified for the active situation handle must reload LoadedSituation to the \
             edited battlefield WITHOUT a restart",
        );
    }

    /// C3: a `Modified` for a DIFFERENT asset id leaves `LoadedSituation` untouched — the
    /// filter is on the ACTIVE handle id only.
    ///
    /// Pin-discriminating: dropping the id filter reloads on any situation asset's event.
    #[test]
    fn modified_event_for_other_id_does_not_reload() {
        let mut app = app();

        let baseline = situation_with_player_faction(0);
        let active = add_asset(&mut app, baseline.clone());
        // A second, unrelated situation asset whose value differs from the active one.
        let other = add_asset(&mut app, situation_with_player_faction(9));
        app.world_mut()
            .insert_resource(LoadedSituation::new(baseline.clone()));
        app.world_mut()
            .insert_resource(ActiveSituationHandle::new(active));

        app.update();
        // Edit the OTHER asset, fire Modified for it only.
        hot_edit(&mut app, &other, situation_with_player_faction(5));
        inject_modified(&mut app, &other);
        app.update();

        let after = app
            .world()
            .get_resource::<LoadedSituation>()
            .map(|s| *s.player_faction);
        assert_eq!(
            after,
            Some(*baseline.player_faction),
            "a Modified for a non-active situation id must NOT reload LoadedSituation",
        );
    }
}
