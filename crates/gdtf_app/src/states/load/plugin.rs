use bevy::{asset::AssetServer, prelude::*};
use gdtf_assets::{HotRonAppExt, RonAssetAppExt};
use gdtf_battle_sim::{
    FieldDef, FieldDefRegistry,
    armor::{ArmorRegistry, ArmorSpec},
    ganger::{GangRegistry, GangRoster},
    injuries::{InjuryDef, InjuryRegistry, InjuryWeighting},
    level::{PrefabRegistry, PrefabSpec, UuidThemeDef, UuidThemeRegistry},
    procgen::ProcgenTuning,
    situation::Situation,
    terrain::def::{TerrainDef, TerrainDefRegistry},
    tuning::{CombatTuning, GangerStatTuning},
    weapon::{
        AttachmentRegistry, AttachmentSpec, MeleeWeaponRegistry, MeleeWeaponSpec, WeaponRegistry,
        WeaponSpec,
    },
};
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec};

use crate::states::{
    AppState,
    load::{
        resources::{LoadHandles, LoadedSituation},
        systems::*,
    },
};

/// Path of the loose authored-situation RON, relative to the asset source root
/// (GTW-205 / E10.3 — the canonical authored battlefield the Generation slice reads).
const SITUATION_RON_PATH: &str = "content/situations/skirmish.ron";

/// Path of the loose combat-tuning RON, relative to the asset source root
/// (GTW-206 / E10.4 — the shipped balance coefficients the sim marches with).
const TUNING_RON_PATH: &str = "core_tuning/combat.tuning.ron";

/// Path of the loose ganger stat-tuning RON, relative to the asset source root
/// (GTW-384 — the attribute → computed-stat derivation weights, a SEPARATE file
/// from `core_tuning/combat.tuning.ron`).
const STAT_TUNING_RON_PATH: &str = "core_tuning/stat.tuning.ron";

/// Path of the loose procgen fill-tuning RON, relative to the asset source root
/// (GTW-533 — the OQ-6 procgen fill knobs the space-packing fill pass reads).
const PROCGEN_TUNING_RON_PATH: &str = "core_tuning/procgen.tuning.ron";

/// The situation chain's [`HotRonMapFn`](gdtf_assets::HotRonMapFn): wrap the
/// deserialized authored [`Situation`] as the persistent [`LoadedSituation`]
/// resource (the mapped hot-RON variant — the payload and the resource are
/// DIFFERENT types; the server goes unused here).
fn map_loaded_situation(situation: &Situation, _asset_server: &AssetServer) -> LoadedSituation {
    LoadedSituation::new(situation.clone())
}

/// The situation chain's `Failed -> default` hook: the EMPTY battlefield, so a
/// bad/missing `content/situations/skirmish.ron` never strands `Load` (the
/// battle then has zero gangers rather than hanging the machine) — inserted
/// ONLY on a genuine `Failed`, never while still loading (the GTW-261
/// empty-battle-race guarantee, preserved by the generic resolve).
fn fallback_loaded_situation() -> LoadedSituation {
    LoadedSituation::new(Situation::default())
}

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
        // DEDICATED `weapon.ron` extension (the files are `assets/content/weapons/ranged/*.weapon.ron`),
        // making the folder dispatch unambiguous regardless of registration order (the
        // contract's "unambiguous .ron loader dispatch" goal). Registered here in
        // `build` BEFORE the `kick_off_loads` `load_folder("weapons")` runs — 0.18
        // extension-dispatch needs the loader registered first.
        if app.world().get_resource::<AssetServer>().is_some() {
            app.init_ron_asset::<GdtfThemeSpec>();
            // GTW-564: the four SINGLE-FILE gate-blocking chains register through the
            // generic hot-RON seam — ONE ext call each wires the `Startup` kick-off (the
            // persistent generic handle), the gated resolve (inserts the resource ONCE, on
            // Loaded — or the fallback on a genuine Failed, so Load never strands), and
            // the ungated live redrive (the GTW-374/GTW-533 hot-reloads, preserved). The
            // `transition_to_intro` gate below still requires every resolved resource, so
            // the Load-gating semantics are unchanged.
            //
            // GTW-261: the situation is the MAPPED variant (Situation payload ->
            // LoadedSituation resource); its Failed fallback is the EMPTY battlefield.
            app.init_hot_ron_resource_mapped_with_fallback::<Situation, LoadedSituation>(
                SITUATION_RON_PATH,
                map_loaded_situation,
                fallback_loaded_situation,
            );
            // GTW-206 (E10.4): the shipped combat tuning (payload IS the resource).
            app.init_hot_ron_resource_with_fallback::<CombatTuning>(
                TUNING_RON_PATH,
                CombatTuning::default,
            );
            // GTW-384: the shipped GangerStatTuning (a SEPARATE file from
            // core_tuning/combat.tuning.ron — the user-directed split). Its redrive's
            // ResMut overwrite trips the sim's `rederive_stats_on_tuning_change`.
            app.init_hot_ron_resource_with_fallback::<GangerStatTuning>(
                STAT_TUNING_RON_PATH,
                GangerStatTuning::default,
            );
            // GTW-533: the shipped ProcgenTuning (the OQ-6 fill knobs), so the next
            // battle GENERATION packs with hot-edited knobs with NO restart.
            app.init_hot_ron_resource_with_fallback::<ProcgenTuning>(
                PROCGEN_TUNING_RON_PATH,
                ProcgenTuning::default,
            );
            app.init_ron_asset_with_extensions::<WeaponSpec>(vec!["weapon.ron"]);
            // GTW-505: the MELEE weapon files mirror the ranged scheme — each loads as a
            // `RonAsset<MeleeWeaponSpec>` via `load_folder`, so it claims its OWN dedicated
            // `melee_weapon.ron` compound extension (files are
            // `assets/content/weapons/melee/*.melee_weapon.ron`), keeping the folder dispatch
            // unambiguous among GDTF's many `.ron` loaders. Registered here in `build` BEFORE the
            // kick-off's `load_folder("content/weapons/melee")` runs.
            app.init_ron_asset_with_extensions::<MeleeWeaponSpec>(vec!["melee_weapon.ron"]);
            // GTW-549 PHASE 1: the data-driven attachment items mirror the weapon scheme —
            // each loads as a `RonAsset<AttachmentSpec>` via `load_folder`, so it claims its
            // OWN dedicated `attachment.ron` compound extension (files are
            // `assets/content/attachments/*.attachment.ron`), keeping the folder dispatch
            // unambiguous among GDTF's many `.ron` loaders. Registered here in `build` BEFORE
            // the kick-off's `load_folder("content/attachments")` runs.
            app.init_ron_asset_with_extensions::<AttachmentSpec>(vec!["attachment.ron"]);
            // GTW-269: armor files mirror the weapon scheme — each loads as a
            // `RonAsset<ArmorSpec>` via `load_folder`, so it claims its OWN dedicated
            // `armor.ron` extension (files are `assets/content/armor/*.armor.ron`) to keep the
            // folder dispatch unambiguous among GDTF's many `.ron` loaders, exactly as
            // the weapon loader does. Registered here in `build` BEFORE the kick-off's
            // `load_folder("armor")` runs.
            app.init_ron_asset_with_extensions::<ArmorSpec>(vec!["armor.ron"]);
            // GTW-545: the area-damage-field catalog files mirror the armor scheme — each loads
            // as a `RonAsset<FieldDef>` via `load_folder`, so it claims its OWN dedicated
            // `field.ron` compound extension (files are `assets/content/fields/*.field.ron`) to
            // keep the folder dispatch unambiguous among GDTF's many `.ron` loaders, exactly as
            // the armor loader does. Registered here in `build` BEFORE the kick-off's
            // `load_folder("content/fields")` runs.
            app.init_ron_asset_with_extensions::<FieldDef>(vec!["field.ron"]);
            // GTW-494 (child T08 of GTW-476): the OLD flat-dir per-file `terrain.ron`
            // and `theme.ron` loaders are RETIRED (and GTW-496 deleted their types). The
            // UUID-model successors — the GTW-487 `TerrainDef` (`terrain_def.ron`) +
            // `UuidThemeDef` (`terrain_theme.ron`) loaders registered below — are now the ONLY
            // terrain / theme resolvers in the Load flow (the sim + procgen + presenter consume
            // the new registries as of GTW-491/492/493). The legacy `terrain.ron` / `theme.ron`
            // extensions are now free for the new model to reclaim in a later slice.
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
            // GTW-489 (child T05c of GTW-476): the UUID-keyed prefab fragments load through the
            // SAME generic RON loader via `load_folder` of the `content/maps/<theme>/<size>/`
            // tree, claiming the dedicated `prefab.ron` compound extension (files are
            // `assets/content/maps/<theme>/<size>/*.prefab.ron`). GTW-494 (child T08): the OLD flat-dir per-file `prefab.ron`
            // loader over `content/maps/` is RETIRED (and GTW-496 deleted its types) — the
            // loader registered here is now the ONLY prefab resolver in the Load flow (the
            // procgen pipeline consumes `PrefabRegistry` as of GTW-492). Registered here in
            // `build` BEFORE the kick-off's `load_folder("content/maps")` runs.
            app.init_ron_asset_with_extensions::<PrefabSpec>(vec!["prefab.ron"]);
            // GTW-487 (child T05a of GTW-476): the NEW UUID-keyed terrain + theme models load
            // through the SAME generic RON loader via `load_folder` of the per-theme `content/terrain/`
            // tree, each claiming its OWN dedicated compound extension — `terrain_def.ron` for
            // a `RonAsset<TerrainDef>` (files `content/terrain/<theme>/<tile>.terrain_def.ron`) and
            // `terrain_theme.ron` for a `RonAsset<UuidThemeDef>` (files
            // `content/terrain/<theme>/<theme>.terrain_theme.ron`). DISTINCT from the legacy
            // `terrain.ron` / `theme.ron` extensions so a second loader on either does NOT
            // clobber the still-live legacy `content/terrain` / `content/themes` folder loads
            // (Bevy dispatches a `load_folder` member by extension alone — see terrain_model's
            // module doc + the C5 constraint). Registered here in `build` BEFORE the kick-off's
            // `load_folder("content/terrain")` runs.
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
                        // GTW-564: the situation + the three single-file tunings left this
                        // or-chain — their generic resolves poll themselves (each gated on
                        // its own handle-present + resource-absent), so the orchestrator
                        // only keeps running for the theme + the FOLDER registries. The
                        // transition gate below still requires ALL of them.
                        not(resource_exists::<GdtfTheme>)
                            .or_else(not(resource_exists::<WeaponRegistry>))
                            // GTW-505: the MeleeWeaponRegistry is a gate-blocking resource too
                            // (every ganger gets a melee weapon; the melee folder must be
                            // verified loaded before Load exits, or a battle fails closed).
                            .or_else(not(resource_exists::<MeleeWeaponRegistry>))
                            // GTW-549 PHASE 1: the AttachmentRegistry is a gate-blocking
                            // resource too — the `content/attachments/` folder must be verified
                            // loaded before Load exits (inserted on success OR failure — no
                            // strand). PHASE 2 resolves each weapon's `attachment_slots` keys
                            // against it.
                            .or_else(not(resource_exists::<AttachmentRegistry>))
                            .or_else(not(resource_exists::<ArmorRegistry>))
                            // GTW-545: the FieldDefRegistry (area-damage-field catalog) is a
                            // gate-blocking resource too — the setup seeds a situation's fields
                            // against it; the `content/fields/` folder must be verified loaded
                            // before Load exits (inserted on success OR failure — no strand).
                            .or_else(not(resource_exists::<FieldDefRegistry>))
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
                            // GTW-489: the UUID-keyed PrefabRegistry is gate-blocking — the v2
                            // prefab fragments must be verified loaded before Load exits. The
                            // procgen pipeline consumes it (GTW-492). The gate only verifies the
                            // folder was walked (the registry is inserted on success OR failure).
                            .or_else(not(resource_exists::<PrefabRegistry>))
                            // GTW-487: the UUID-keyed TerrainDefRegistry + UuidThemeRegistry are
                            // gate-blocking — the per-theme `content/terrain/` folder must be verified
                            // loaded before Load exits. The sim + procgen + presenter consume
                            // them (GTW-491/492/493). The gate only verifies the folder was
                            // walked (the registries are inserted on success OR failure).
                            .or_else(not(resource_exists::<TerrainDefRegistry>))
                            .or_else(not(resource_exists::<UuidThemeRegistry>)),
                    ),
            ),
            // Once a GdtfTheme, a CombatTuning, a WeaponRegistry, a LoadedSituation, an
            // ArmorRegistry, AND the UUID-keyed TerrainDefRegistry / UuidThemeRegistry /
            // PrefabRegistry all exist, leave Load for Intro (GTW-206 / E10.4 AC5: theme +
            // tuning required; GTW-257: the WeaponRegistry too; GTW-261: the LoadedSituation
            // too, so a battle never starts before its real situation loads — the empty-battle
            // race fix; GTW-269: the ArmorRegistry too; GTW-487 / GTW-489 / GTW-494: the
            // UUID-keyed terrain / theme / prefab registries too, so the per-theme `content/terrain/`
            // + `maps/` folders are verified loaded before Load exits — the sim + procgen +
            // presenter consume them as of GTW-491/492/493). On a failed situation the resolve
            // falls back to an empty LoadedSituation, and failed folder loads fall back to
            // empty registries, so a slow/failed asset still never strands Load (the no-strand
            // guarantee preserved via the failure fallback).
            transition_to_intro.run_if(
                in_state(AppState::Load)
                    .and_then(resource_exists::<GdtfTheme>)
                    .and_then(resource_exists::<CombatTuning>)
                    // GTW-384: the GangerStatTuning must be present before Load exits, so a
                    // battle never starts before its stat-derivation tuning loads.
                    .and_then(resource_exists::<GangerStatTuning>)
                    // GTW-533: the ProcgenTuning must be present before Load exits, so the
                    // first battle's terrain generation reads the loaded fill knobs (not the
                    // default) — the shipped `core_tuning/procgen.tuning.ron` is verified
                    // loaded here.
                    .and_then(resource_exists::<ProcgenTuning>)
                    .and_then(resource_exists::<WeaponRegistry>)
                    // GTW-505: the MeleeWeaponRegistry must be present before Load exits, so the
                    // melee weapons folder is verified loaded before any battle arms melee.
                    .and_then(resource_exists::<MeleeWeaponRegistry>)
                    // GTW-549 PHASE 1: the AttachmentRegistry must be present before Load
                    // exits, so the `content/attachments/` folder is verified loaded before
                    // any battle resolves a weapon's attachment keys.
                    .and_then(resource_exists::<AttachmentRegistry>)
                    .and_then(resource_exists::<LoadedSituation>)
                    .and_then(resource_exists::<ArmorRegistry>)
                    // GTW-545: the FieldDefRegistry must be present before Load exits, so the
                    // `content/fields/` catalog is verified loaded before any battle seeds a
                    // field.
                    .and_then(resource_exists::<FieldDefRegistry>)
                    // GTW-437: the InjuryRegistry must be present before Load exits, so the
                    // injuries folder is verified loaded before the GTW-438 roll uses it.
                    .and_then(resource_exists::<InjuryRegistry>)
                    // GTW-415: the GangRegistry must be present before Load exits, so the
                    // gangs folder is verified loaded before any battle resolves a placed
                    // ganger's (gang, member) ref against it.
                    .and_then(resource_exists::<GangRegistry>)
                    // GTW-489: the PrefabRegistry must be present before Load exits, so the v2
                    // prefab fragments are verified loaded before the GTW-492 v2 assembler reads
                    // them.
                    .and_then(resource_exists::<PrefabRegistry>)
                    // GTW-487: the TerrainDefRegistry + UuidThemeRegistry must be present before
                    // Load exits, so the per-theme `content/terrain/` folder is verified loaded before
                    // the GTW-491/492/493 consumers read them.
                    .and_then(resource_exists::<TerrainDefRegistry>)
                    .and_then(resource_exists::<UuidThemeRegistry>),
            ),
        )
            .chain(),
    )
    .add_systems(OnExit(AppState::Load), (print_on_exit, cleanup).chain());
}

/// GTW-374: register the LIVE content/tuning hot-reload handlers in an UNGATED `Update` so
/// they react to a `core_tuning/*.tuning.ron` / `weapons/*.weapon.ron` / `armor/*.armor.ron`
/// / `situations/*.ron` / … file edit AFTER `Load` has exited (the data persists, the
/// `LoadHandles` do not — hence the persistent `Active*Handle` resources each handler
/// reads). GTW-533 audited + completed the set so every shipped `.ron` content/tuning +
/// sprite asset the GAME hosts hot-reloads through this ONE shared mechanism (Bevy's
/// `file_watcher`), adding the situation, keybinds, and procgen-tuning handlers. The EDITOR
/// is a SECOND in-app asset host (the separate `gdtf_content_editor` binary); its half of the
/// same "game AND editor, no restart" contract is wired independently in the editor's OWN
/// `register_load` (`crates/gdtf_content_editor/src/load/`), reusing this SAME `file_watcher`
/// mechanism — this handler set is the GAME's coverage only. Each handler self-guards on its
/// `Option`al borrows (`bevy-traps.md` #1), so it is a harmless no-op until the load chain has
/// resolved the resource it re-derives.
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
            // GTW-564: the situation + combat / stat / procgen tuning redrives moved
            // onto the generic hot-RON seam (registered by the ext calls in `build`);
            // this set now carries the FOLDER-registry redrives only.
            redrive_weapons_on_asset_event,
            // GTW-505: the melee weapon hot-reload — rebuilds the MeleeWeaponRegistry on a
            // `weapons/melee/*.melee_weapon.ron` edit, mirroring the ranged hot-reload.
            redrive_melee_weapons_on_asset_event,
            // GTW-549 PHASE 1: the attachment hot-reload — rebuilds the AttachmentRegistry on a
            // `content/attachments/*.attachment.ron` edit, mirroring the melee/weapon
            // hot-reload pattern (one folder, one registry).
            redrive_attachments_on_asset_event,
            redrive_armor_on_asset_event,
            // GTW-545: the area-damage-field hot-reload — rebuilds the FieldDefRegistry catalog
            // on a `content/fields/*.field.ron` edit, mirroring the weapon/armor hot-reload
            // pattern (one folder, one registry).
            redrive_fields_on_asset_event,
            // GTW-437: the injury hot-reload — rebuilds BOTH the InjuryRegistry and the
            // InjuryTables on an edit to ANY `injuries/**/*.injury.ron` OR `*.weighting.ron`,
            // mirroring the weapon/armor hot-reload pattern (one folder, two resources).
            redrive_injuries_on_asset_event,
            // GTW-415: the gang hot-reload — rebuilds the GangRegistry on a
            // `gangs/*.gang.ron` edit, mirroring the weapon/armor hot-reload pattern.
            redrive_gangs_on_asset_event,
            // GTW-489: the prefab hot-reload — rebuilds the UUID-keyed PrefabRegistry on a
            // `maps/**/*.prefab.ron` edit (NO edge-opening validation — the schema has none),
            // mirroring the gang hot-reload pattern.
            redrive_prefabs_on_asset_event,
            // GTW-487: the terrain-def + theme-def hot-reloads — rebuild the UUID-keyed
            // TerrainDefRegistry / UuidThemeRegistry on an edit to ANY
            // `content/terrain/**/*.terrain_def.ron` / `*.terrain_theme.ron`, mirroring the legacy
            // terrain/theme hot-reload pattern (one folder, two registries).
            redrive_terrain_defs_on_asset_event,
            redrive_theme_defs_on_asset_event,
        ),
    );
}
