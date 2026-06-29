use bevy::{asset::AssetServer, prelude::*};
use gdtf_assets::RonAssetAppExt;
use gdtf_battle_sim::{
    armor::{ArmorRegistry, ArmorSpec},
    ganger::{GangRegistry, GangRoster},
    injuries::{InjuryDef, InjuryRegistry, InjuryWeighting},
    level::{
        PrefabRegistry, PrefabSpec, ThemeCatalogRegistry, ThemeSpec, UuidThemeDef,
        UuidThemeRegistry,
    },
    situation::Situation,
    terrain::{
        def::{TerrainDef, TerrainDefRegistry},
        piece::{TerrainRegistry, TerrainSpec},
    },
    tuning::{CombatTuning, GangerStatTuning},
    weapon::{WeaponRegistry, WeaponSpec},
};
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec};

use crate::states::{
    AppState,
    load::{
        resources::{LoadHandles, LoadedSituation},
        systems::*,
    },
};

pub(in crate::states) struct LoadScenePlugin;

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
        // DEDICATED `weapon.ron` extension (the files are `assets/content/weapons/*.weapon.ron`),
        // making the folder dispatch unambiguous regardless of registration order (the
        // contract's "unambiguous .ron loader dispatch" goal). Registered here in
        // `build` BEFORE the `kick_off_loads` `load_folder("weapons")` runs — 0.18
        // extension-dispatch needs the loader registered first.
        if app.world().get_resource::<AssetServer>().is_some() {
            app.init_ron_asset::<GdtfThemeSpec>();
            app.init_ron_asset::<Situation>();
            app.init_ron_asset::<CombatTuning>();
            // GTW-384: the shipped GangerStatTuning loads through the SAME generic RON
            // loader (a SEPARATE file from core_tuning/combat.tuning.ron — the user-directed split),
            // registered here behind the one AssetServer guard alongside the others.
            app.init_ron_asset::<GangerStatTuning>();
            app.init_ron_asset_with_extensions::<WeaponSpec>(vec!["weapon.ron"]);
            // GTW-269: armor files mirror the weapon scheme — each loads as a
            // `RonAsset<ArmorSpec>` via `load_folder`, so it claims its OWN dedicated
            // `armor.ron` extension (files are `assets/content/armor/*.armor.ron`) to keep the
            // folder dispatch unambiguous among GDTF's many `.ron` loaders, exactly as
            // the weapon loader does. Registered here in `build` BEFORE the kick-off's
            // `load_folder("armor")` runs.
            app.init_ron_asset_with_extensions::<ArmorSpec>(vec!["armor.ron"]);
            // GTW-394: terrain files mirror the armor/weapon scheme — each loads as a
            // `RonAsset<TerrainSpec>` via `load_folder`, claiming its OWN dedicated
            // `terrain.ron` compound extension (files are `assets/content/terrain/*.terrain.ron`)
            // to keep the folder dispatch unambiguous among GDTF's many `.ron` loaders.
            // Registered here in `build` BEFORE the kick-off's `load_folder("terrain")`
            // runs.
            app.init_ron_asset_with_extensions::<TerrainSpec>(vec!["terrain.ron"]);
            // GTW-409: theme catalogs mirror the terrain/armor/weapon scheme — each loads
            // as a `RonAsset<ThemeSpec>` via `load_folder`, claiming its OWN dedicated
            // `theme.ron` compound extension (files are `assets/content/themes/*.theme.ron`)
            // to keep the folder dispatch unambiguous among GDTF's many `.ron` loaders.
            // Registered here in `build` BEFORE the kick-off's `load_folder("themes")` runs.
            app.init_ron_asset_with_extensions::<ThemeSpec>(vec!["theme.ron"]);
            // GTW-437: the injuries folder carries TWO asset types, each via the SAME
            // generic RON loader but loaded by `load_folder` (extension dispatch). Each
            // claims its OWN dedicated compound extension — `injury.ron` for the per-injury
            // defs (files are `assets/content/injuries/<part>/*.injury.ron`) and `weighting.ron`
            // for the per-part weighting tables (`assets/content/injuries/weighting/*.weighting.ron`)
            // — so the recursive folder dispatch is unambiguous among GDTF's many `.ron`
            // loaders. Registered here in `build` BEFORE the kick-off's
            // `load_folder("injuries")` runs.
            app.init_ron_asset_with_extensions::<InjuryDef>(vec!["injury.ron"]);
            app.init_ron_asset_with_extensions::<InjuryWeighting>(vec!["weighting.ron"]);
            // GTW-415: gang rosters mirror the weapon/armor/terrain scheme — each loads as
            // a `RonAsset<GangRoster>` via `load_folder`, claiming its OWN dedicated
            // `gang.ron` compound extension (files are `assets/content/gangs/*.gang.ron`) to
            // keep the folder dispatch unambiguous among GDTF's many `.ron` loaders.
            // Registered here in `build` BEFORE the kick-off's `load_folder("gangs")` runs.
            app.init_ron_asset_with_extensions::<GangRoster>(vec!["gang.ron"]);
            // GTW-418: prefab fragments mirror the gangs/weapon/armor/terrain scheme —
            // each loads as a `RonAsset<PrefabSpec>` via `load_folder` (recursive, over the
            // NESTED `assets/content/maps/<theme>/<size>/` tree), claiming its OWN dedicated
            // `prefab.ron` compound extension (files are
            // `assets/content/maps/<theme>/<size>/*.prefab.ron`) to keep the folder dispatch
            // unambiguous among GDTF's many `.ron` loaders. Registered here in `build`
            // BEFORE the kick-off's `load_folder("content/maps")` runs.
            app.init_ron_asset_with_extensions::<PrefabSpec>(vec!["prefab.ron"]);
            // GTW-487 (child T05a of GTW-476): the NEW UUID-keyed terrain + theme models load
            // through the SAME generic RON loader via `load_folder` of the per-theme `terrain/`
            // tree, each claiming its OWN dedicated compound extension — `terrain_def.ron` for
            // a `RonAsset<TerrainDef>` (files `terrain/<theme>/<tile>.terrain_def.ron`) and
            // `terrain_theme.ron` for a `RonAsset<UuidThemeDef>` (files
            // `terrain/<theme>/<theme>.terrain_theme.ron`). DISTINCT from the legacy
            // `terrain.ron` / `theme.ron` extensions so a second loader on either does NOT
            // clobber the still-live legacy `content/terrain` / `content/themes` folder loads
            // (Bevy dispatches a `load_folder` member by extension alone — see terrain_model's
            // module doc + the C5 constraint). Registered here in `build` BEFORE the kick-off's
            // `load_folder("terrain")` runs.
            app.init_ron_asset_with_extensions::<TerrainDef>(vec!["terrain_def.ron"]);
            app.init_ron_asset_with_extensions::<UuidThemeDef>(vec!["terrain_theme.ron"]);
            add_hot_reload_systems(app);
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
                            // GTW-384: the GangerStatTuning is a gate-blocking resource too
                            // (the sim derives every ganger's stats from it).
                            .or_else(not(resource_exists::<GangerStatTuning>))
                            .or_else(not(resource_exists::<WeaponRegistry>))
                            .or_else(not(resource_exists::<LoadedSituation>))
                            .or_else(not(resource_exists::<ArmorRegistry>))
                            // GTW-394: the TerrainRegistry is a gate-blocking resource too
                            // (the downstream generation epic uses it; the folder must be
                            // verified loaded before Load exits).
                            .or_else(not(resource_exists::<TerrainRegistry>))
                            // GTW-409: the ThemeCatalogRegistry is a gate-blocking resource
                            // too (the editor/procgen children use it; the themes folder
                            // must be verified loaded before Load exits).
                            .or_else(not(resource_exists::<ThemeCatalogRegistry>))
                            // GTW-437: the InjuryRegistry is a gate-blocking resource too
                            // (the GTW-438 roll uses it + the InjuryTables; the injuries
                            // folder must be verified loaded before Load exits).
                            .or_else(not(resource_exists::<InjuryRegistry>))
                            // GTW-415: the GangRegistry is a gate-blocking resource too
                            // (the v2 setup_battle resolves every placed ganger's
                            // (gang, member) ref against it; the gangs folder must be
                            // verified loaded before Load exits, or a real battle fails
                            // closed with GangNotFound).
                            .or_else(not(resource_exists::<GangRegistry>))
                            // GTW-418: the PrefabRegistry is a gate-blocking resource too
                            // (the GTW-424 assembler packs its fragments; the maps folder
                            // must be verified loaded before Load exits).
                            .or_else(not(resource_exists::<PrefabRegistry>))
                            // GTW-487: the NEW UUID-keyed TerrainDefRegistry +
                            // UuidThemeRegistry are gate-blocking too — the new per-theme
                            // `terrain/` folder must be verified loaded before Load exits.
                            // They resolve EMPTY against un-migrated shipped content (the
                            // designed fail-closed state); the gate only verifies the folder
                            // was walked (the registries are inserted on success OR failure).
                            .or_else(not(resource_exists::<TerrainDefRegistry>))
                            .or_else(not(resource_exists::<UuidThemeRegistry>)),
                    ),
            ),
            // Once a GdtfTheme, a CombatTuning, a WeaponRegistry, a LoadedSituation,
            // an ArmorRegistry, AND a TerrainRegistry all exist, leave Load for Intro
            // (GTW-206 / E10.4 AC5: theme + tuning required; GTW-257: the WeaponRegistry
            // too; GTW-261: the LoadedSituation too, so a battle never starts before its
            // real situation loads — the empty-battle race fix; GTW-269: the ArmorRegistry
            // too, so the armor folder is verified loaded before Load exits — the registry
            // is DORMANT this slice, consumed by slice C; GTW-394: the TerrainRegistry
            // too, so the terrain folder is verified loaded before Load exits — the registry
            // is DORMANT this slice, consumed by the generation epic). On a failed situation
            // the resolve falls back to an empty LoadedSituation, and failed folder loads
            // fall back to empty registries, so a slow/failed asset still never strands
            // Load (the no-strand guarantee preserved via the failure fallback).
            transition_to_intro.run_if(
                in_state(AppState::Load)
                    .and_then(resource_exists::<GdtfTheme>)
                    .and_then(resource_exists::<CombatTuning>)
                    // GTW-384: the GangerStatTuning must be present before Load exits, so a
                    // battle never starts before its stat-derivation tuning loads.
                    .and_then(resource_exists::<GangerStatTuning>)
                    .and_then(resource_exists::<WeaponRegistry>)
                    .and_then(resource_exists::<LoadedSituation>)
                    .and_then(resource_exists::<ArmorRegistry>)
                    // GTW-394: the TerrainRegistry must be present before Load exits, so the
                    // terrain folder is verified loaded before any downstream use.
                    .and_then(resource_exists::<TerrainRegistry>)
                    // GTW-409: the ThemeCatalogRegistry must be present before Load exits, so
                    // the themes folder is verified loaded before any editor/procgen use.
                    .and_then(resource_exists::<ThemeCatalogRegistry>)
                    // GTW-437: the InjuryRegistry must be present before Load exits, so the
                    // injuries folder is verified loaded before the GTW-438 roll uses it.
                    .and_then(resource_exists::<InjuryRegistry>)
                    // GTW-415: the GangRegistry must be present before Load exits, so the
                    // gangs folder is verified loaded before any battle resolves a placed
                    // ganger's (gang, member) ref against it.
                    .and_then(resource_exists::<GangRegistry>)
                    // GTW-418: the PrefabRegistry must be present before Load exits, so the
                    // maps folder is verified loaded before the GTW-424 assembler uses it.
                    .and_then(resource_exists::<PrefabRegistry>)
                    // GTW-487: the NEW TerrainDefRegistry + UuidThemeRegistry must be present
                    // before Load exits, so the new per-theme `terrain/` folder is verified
                    // loaded before any later GTW-476 consumer reads them.
                    .and_then(resource_exists::<TerrainDefRegistry>)
                    .and_then(resource_exists::<UuidThemeRegistry>),
            ),
        )
            .chain(),
    )
    .add_systems(OnExit(AppState::Load), (print_on_exit, cleanup).chain());
}

/// GTW-374: register the three LIVE combat-data hot-reload handlers in an UNGATED
/// `Update` so they react to a `core_tuning/combat.tuning.ron` / `weapons/*.weapon.ron` /
/// `armor/*.armor.ron` file edit AFTER `Load` has exited (the data persists, the
/// `LoadHandles` do not — hence the persistent `Active*Handle` resources each handler
/// reads). Each handler self-guards on its `Option`al borrows (`bevy-traps.md` #1), so
/// it is a harmless no-op until the load chain has resolved the resource it re-derives.
///
/// Called only inside the `AssetServer`-present guard: the
/// `Messages<AssetEvent<RonAsset<T>>>` buffers these `MessageReader`s need are
/// registered by `init_ron_asset` (above), so a `MinimalPlugins` headless app that
/// skips the loader registration also skips these systems (`bevy-traps.md` #4 — a
/// `MessageReader` panics validation without its buffer).
fn add_hot_reload_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            redrive_combat_tuning_on_asset_event,
            // GTW-384: the ganger stat-tuning hot-reload — overwrites the GangerStatTuning
            // resource on a `core_tuning/stat.tuning.ron` edit, whose Changed<GangerStatTuning>
            // trips the sim's `rederive_stats_on_tuning_change`.
            redrive_stat_tuning_on_asset_event,
            redrive_weapons_on_asset_event,
            redrive_armor_on_asset_event,
            // GTW-394: the terrain hot-reload — overwrites the TerrainRegistry resource on
            // a `terrain/*.terrain.ron` edit, mirroring the weapon/armor hot-reload pattern.
            redrive_terrain_on_asset_event,
            // GTW-409: the theme-catalog hot-reload — rebuilds the ThemeCatalogRegistry on a
            // `themes/*.theme.ron` edit, mirroring the terrain hot-reload pattern.
            redrive_themes_on_asset_event,
            // GTW-437: the injury hot-reload — rebuilds BOTH the InjuryRegistry and the
            // InjuryTables on an edit to ANY `injuries/**/*.injury.ron` OR `*.weighting.ron`,
            // mirroring the weapon/armor hot-reload pattern (one folder, two resources).
            redrive_injuries_on_asset_event,
            // GTW-415: the gang hot-reload — rebuilds the GangRegistry on a
            // `gangs/*.gang.ron` edit, mirroring the weapon/armor hot-reload pattern.
            redrive_gangs_on_asset_event,
            // GTW-418: the prefab hot-reload — rebuilds the PrefabRegistry on a
            // `maps/**/*.prefab.ron` edit (re-running the C6 edge-opening validation),
            // mirroring the gang hot-reload pattern.
            redrive_prefabs_on_asset_event,
            // GTW-487: the NEW terrain-def + theme-def hot-reloads — rebuild the UUID-keyed
            // TerrainDefRegistry / UuidThemeRegistry on an edit to ANY
            // `terrain/**/*.terrain_def.ron` / `*.terrain_theme.ron`, mirroring the legacy
            // terrain/theme hot-reload pattern (one folder, two registries).
            redrive_terrain_defs_on_asset_event,
            redrive_theme_defs_on_asset_event,
        ),
    );
}
