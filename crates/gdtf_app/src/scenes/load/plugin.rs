use bevy::{asset::AssetServer, prelude::*};
use gdtf_assets::RonAssetAppExt;
use gdtf_battle_sim::{
    armor::{ArmorRegistry, ArmorSpec},
    situation::Situation,
    tuning::CombatTuning,
    weapon::{WeaponRegistry, WeaponSpec},
};
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec};

use crate::{
    scenes::load::{
        resources::{LoadHandles, LoadedSituation},
        systems::*,
    },
    states::AppState,
};

pub(in crate::scenes) struct LoadScenePlugin;

impl Plugin for LoadScenePlugin {
    fn build(&self, app: &mut App) {
        // Register the generic RON loader for the theme spec once (GTW-136):
        // this installs `Assets<RonAsset<GdtfThemeSpec>>` and its loader so the
        // kick-off can `asset_server.load::<RonAsset<GdtfThemeSpec>>(..)`.
        //
        // `init_asset` immediately requires the `AssetServer` resource, so it
        // panics under `MinimalPlugins` (no `AssetPlugin`). Guard on the server's
        // presence: in the production app and the real-asset harness the server
        // exists and the loader registers; in the `MinimalPlugins` state-machine
        // harness it is absent and `kick_off_loads` already no-ops, so skipping
        // the registration there is correct (bevy-traps rule 1).
        //
        // GTW-205 (E10.3): the authored `Situation` loads through the SAME generic
        // RON loader, so register `Assets<RonAsset<Situation>>` + its loader behind
        // the same `asset_server.is_some()` guard as the theme.
        //
        // GTW-206 (E10.4): the shipped `CombatTuning` loads through the SAME generic
        // RON loader too, so register `Assets<RonAsset<CombatTuning>>` + its loader
        // inside this one guard alongside the theme and situation (one guard, three
        // registrations, each registered exactly once).
        //
        // GTW-257: each weapon file loads through the SAME generic RON loader as a
        // `RonAsset<WeaponSpec>`, but via `load_folder` (an UNTYPED, extension-based
        // load) — which Bevy dispatches to the LAST-registered loader for the file's
        // extension. GDTF registers many `ron` loaders (theme / situation / tuning /
        // keybinds / tile-roles / …), so a `load_folder` of plain `.ron` weapon files
        // would be typed non-deterministically. The weapon loader therefore claims a
        // DEDICATED `weapon.ron` extension (the files are `assets/weapons/*.weapon.ron`),
        // making the folder dispatch unambiguous regardless of registration order (the
        // contract's "unambiguous .ron loader dispatch" goal). Registered here in
        // `build` BEFORE the `kick_off_loads` `load_folder("weapons")` runs — 0.18
        // extension-dispatch needs the loader registered first.
        if app.world().get_resource::<AssetServer>().is_some() {
            app.init_ron_asset::<GdtfThemeSpec>();
            app.init_ron_asset::<Situation>();
            app.init_ron_asset::<CombatTuning>();
            app.init_ron_asset_with_extensions::<WeaponSpec>(vec!["weapon.ron"]);
            // GTW-269: armor files mirror the weapon scheme — each loads as a
            // `RonAsset<ArmorSpec>` via `load_folder`, so it claims its OWN dedicated
            // `armor.ron` extension (files are `assets/armor/*.armor.ron`) to keep the
            // folder dispatch unambiguous among GDTF's many `.ron` loaders, exactly as
            // the weapon loader does. Registered here in `build` BEFORE the kick-off's
            // `load_folder("armor")` runs.
            app.init_ron_asset_with_extensions::<ArmorSpec>(vec!["armor.ron"]);
        }
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(
        OnEnter(AppState::Load),
        (print_on_enter, kick_off_loads).chain(),
    )
    .add_systems(
        Update,
        (
            // poll/resolve runs until a GdtfTheme, a CombatTuning, a WeaponRegistry, a
            // LoadedSituation, AND an ArmorRegistry are all inserted (success path
            // resolves the loaded spec/payload/folder/situation; failure path inserts
            // the const default). It runs while ANY required resource is still missing —
            // the theme branch, the GTW-206 tuning branch, the GTW-257 weapons branch,
            // the GTW-261 situation branch, and the GTW-269 armor branch each re-gate
            // internally on their own resource's absence, so none starves another
            // (bevy-traps rule 3). Ordered BEFORE the transition so all five are present
            // when the transition checks for them.
            poll_and_resolve.run_if(
                in_state(AppState::Load)
                    .and_then(resource_exists::<LoadHandles>)
                    .and_then(
                        not(resource_exists::<GdtfTheme>)
                            .or_else(not(resource_exists::<CombatTuning>))
                            .or_else(not(resource_exists::<WeaponRegistry>))
                            .or_else(not(resource_exists::<LoadedSituation>))
                            .or_else(not(resource_exists::<ArmorRegistry>)),
                    ),
            ),
            // Once a GdtfTheme, a CombatTuning, a WeaponRegistry, a LoadedSituation,
            // AND an ArmorRegistry all exist, leave Load for Intro (GTW-206 / E10.4 AC5:
            // theme + tuning required; GTW-257: the WeaponRegistry too; GTW-261: the
            // LoadedSituation too, so a battle never starts before its real situation
            // loads — the empty-battle race fix; GTW-269: the ArmorRegistry too, so the
            // armor folder is verified loaded before Load exits — the registry is
            // DORMANT this slice, consumed by slice C). On a failed situation the
            // resolve falls back to an empty LoadedSituation, and a failed armor folder
            // falls back to an empty ArmorRegistry, so a slow/failed asset still never
            // strands Load (the no-strand guarantee preserved via the failure fallback).
            transition_to_intro.run_if(
                in_state(AppState::Load)
                    .and_then(resource_exists::<GdtfTheme>)
                    .and_then(resource_exists::<CombatTuning>)
                    .and_then(resource_exists::<WeaponRegistry>)
                    .and_then(resource_exists::<LoadedSituation>)
                    .and_then(resource_exists::<ArmorRegistry>),
            ),
        )
            .chain(),
    )
    .add_systems(OnExit(AppState::Load), (print_on_exit, cleanup).chain());
}
