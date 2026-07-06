use bevy::{asset::AssetServer, prelude::*};
use gdtf_assets::{ContentFamilyAppExt, ContentValidationDone, HotRonAppExt, RonAssetAppExt};
use gdtf_battle_sim::{
    FieldDefRegistry,
    armor::ArmorRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::{InjuryDef, InjuryRegistry, InjuryWeighting},
    level::{PrefabRegistry, PrefabSpec, UuidThemeRegistry},
    procgen::ProcgenTuning,
    situation::Situation,
    terrain::def::TerrainDefRegistry,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::{
    ArmorFamily, AttachmentsFamily, FieldsFamily, GangsFamily, MeleeWeaponsFamily,
    TerrainDefsFamily, ThemeDefsFamily, WeaponsFamily,
    injuries::{INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION},
    prefabs::PREFAB_EXTENSION,
};
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec};

use crate::states::{
    AppState,
    load::{
        resources::{LoadHandles, LoadedSituation},
        systems::*,
    },
    scaffold::{SceneLabel, log_scene_enter, log_scene_exit},
};

/// Path of the loose authored-situation RON, relative to the asset source root
/// (GTW-205 / E10.3 — the canonical authored battlefield the Generation slice
/// reads). `pub(in crate::states::load)` so the GTW-582 reference-integrity
/// checks can name the situation file in their findings without duplicating the
/// literal.
pub(in crate::states::load) const SITUATION_RON_PATH: &str = "content/situations/skirmish.ron";

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
        // GTW-257 (folder-dispatch rationale): a `load_folder` is an UNTYPED,
        // extension-based load — Bevy dispatches each member to the LAST-registered
        // loader for its extension. GDTF registers many `ron` loaders, so every
        // folder-loaded content type claims a DEDICATED compound extension (e.g.
        // `weapon.ron`), making the dispatch unambiguous regardless of registration
        // order. GTW-570: for the seven generic content families that dedicated
        // extension now rides the `register_content_family` call (each family names
        // its own; GTW-619 moved the attachments folder onto the same seam, the
        // eighth); the bespoke folder loaders below still register theirs by hand.
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
            // GTW-437: the injuries folder carries TWO asset types, each via the SAME
            // generic RON loader but loaded by `load_folder` (extension dispatch). Each
            // claims its OWN dedicated compound extension — `injury.ron` for the per-injury
            // defs (files are `assets/content/injuries/<part>/*.injury.ron`) and `weighting.ron`
            // for the per-part weighting tables (`assets/content/injuries/weighting/*.weighting.ron`)
            // — so the recursive folder dispatch is unambiguous among GDTF's many `.ron`
            // loaders. Registered here in `build` BEFORE the kick-off's
            // `load_folder("injuries")` runs.
            app.init_ron_asset_with_extensions::<InjuryDef>(vec![INJURY_DEF_EXTENSION]);
            app.init_ron_asset_with_extensions::<InjuryWeighting>(vec![INJURY_WEIGHTING_EXTENSION]);
            // GTW-489 (child T05c of GTW-476): the UUID-keyed prefab fragments load through the
            // SAME generic RON loader via `load_folder` of the `content/maps/<theme>/<size>/`
            // tree, claiming the dedicated `prefab.ron` compound extension (files are
            // `assets/content/maps/<theme>/<size>/*.prefab.ron`). GTW-494 (child T08): the OLD flat-dir per-file `prefab.ron`
            // loader over `content/maps/` is RETIRED (and GTW-496 deleted its types) — the
            // loader registered here is now the ONLY prefab resolver in the Load flow (the
            // procgen pipeline consumes `PrefabRegistry` as of GTW-492). Registered here in
            // `build` BEFORE the kick-off's `load_folder("content/maps")` runs.
            app.init_ron_asset_with_extensions::<PrefabSpec>(vec![PREFAB_EXTENSION]);
            add_hot_reload_systems(app);
        }
        // GTW-570: the FOLDER-loaded content families register through the generic
        // content-family seam — ONE ext call each wires the dedicated-extension
        // loader, the `Startup` folder kick-off (the persistent generic
        // `ContentFolderHandle`), the gated resolve (inserts the registry ONCE, on
        // Loaded — or the EMPTY registry on a genuine Failed, so Load never
        // strands), and the ungated live redrive (the GTW-374 hot-reloads,
        // preserved). The `transition_to_intro` gate below still requires every
        // resolved registry, so the Load-gating semantics are unchanged (C3).
        //
        // Deliberately OUTSIDE the `AssetServer` guard (GTW-629): the seam
        // SELF-GATES — with a server it wires the full chain; without one it seeds
        // the family's default registry as the headless fallback instead, so a
        // `MinimalPlugins` walk keeps traversing `Load` with zero per-family seed
        // arms anywhere else.
        app.register_content_family::<WeaponsFamily>();
        app.register_content_family::<MeleeWeaponsFamily>();
        app.register_content_family::<ArmorFamily>();
        app.register_content_family::<FieldsFamily>();
        app.register_content_family::<GangsFamily>();
        // The terrain + theme defs are the PAYLOAD-KEYED families sharing the ONE
        // MIXED `content/terrain/` tree — each walk skips the other family's
        // members via the seam's unconditional TypeId filter (GTW-487 precedent).
        app.register_content_family::<TerrainDefsFamily>();
        app.register_content_family::<ThemeDefsFamily>();
        // GTW-619: the data-driven attachment items (GTW-549) ride the SAME generic
        // seam — the ext call claims their dedicated `attachment.ron` compound
        // extension (files are `assets/content/attachments/*.attachment.ron`), so the
        // bespoke loader registration + kick-off / resolve / redrive chain is gone.
        app.register_content_family::<AttachmentsFamily>();
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Load");
    // GTW-582: install the unified end-of-Load reference-integrity pass — the
    // per-edge check hooks, the seam's Check→Publish plumbing, and the window
    // condition that opens it once every graph registry has resolved. Registered
    // UNCONDITIONALLY (not inside the AssetServer guard): under MinimalPlugins
    // the seeded gate resources open the window, so the ContentValidationDone
    // gate below still releases every headless walk.
    add_content_validation(app);
    app.add_systems(
        OnEnter(AppState::Load),
        (log_scene_enter(label), kick_off_loads).chain(),
    )
    .add_systems(
        Update,
        (
            // poll/resolve runs until the theme + the BESPOKE folder registries
            // (injuries / prefabs) are all inserted (success path resolves the
            // loaded spec/folder; failure path inserts the const default / empty
            // registry). It runs while ANY of those is still missing — each
            // branch re-gates internally on its own resource's absence, so none
            // starves another (bevy-traps rule 3). Ordered BEFORE the transition
            // so all are present when the transition checks. The GTW-564
            // single-file chains and the GTW-570/GTW-619 content families poll
            // themselves through their generic resolves.
            poll_and_resolve.run_if(
                in_state(AppState::Load)
                    .and_then(resource_exists::<LoadHandles>)
                    .and_then(
                        // GTW-564: the situation + the three single-file tunings left this
                        // or-chain — their generic resolves poll themselves (each gated on
                        // its own handle-present + resource-absent). GTW-570 moved the
                        // seven folder families out the same way (GTW-619 moved the
                        // attachments too), so the orchestrator only keeps running for
                        // the theme + the BESPOKE folder registries (injuries /
                        // prefabs). The transition gate below still requires ALL of them.
                        not(resource_exists::<GdtfTheme>)
                            // GTW-437: the InjuryRegistry is a gate-blocking resource too
                            // (the GTW-438 roll uses it + the InjuryTables; the injuries
                            // folder must be verified loaded before Load exits).
                            .or_else(not(resource_exists::<InjuryRegistry>))
                            // GTW-489: the UUID-keyed PrefabRegistry is gate-blocking — the v2
                            // prefab fragments must be verified loaded before Load exits. The
                            // procgen pipeline consumes it (GTW-492). The gate only verifies the
                            // folder was walked (the registry is inserted on success OR failure).
                            .or_else(not(resource_exists::<PrefabRegistry>)),
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
                    // any battle resolves a weapon's attachment keys (published by the
                    // GTW-619 generic content-family resolve; the gate is unchanged).
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
                    .and_then(resource_exists::<UuidThemeRegistry>)
                    // GTW-582: the unified reference-integrity pass must have PUBLISHED its
                    // consolidated report before Load exits — the pass's done-marker is the
                    // explicit ordering that puts validation strictly before this transition.
                    // The publish stamps it UNCONDITIONALLY (findings are loud, never fatal),
                    // so validation can never strand Load (the no-strand guarantee holds).
                    .and_then(resource_exists::<ContentValidationDone>),
            ),
        )
            .chain(),
    )
    .add_systems(
        OnExit(AppState::Load),
        (log_scene_exit(label), cleanup).chain(),
    );
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
            // onto the generic hot-RON seam; GTW-570 moved the seven folder-family
            // redrives (weapons / melee / armor / fields / gangs / terrain + theme
            // defs) onto the generic content-family seam, and GTW-619 moved the
            // attachments redrive the same way (all registered by the ext calls in
            // `build`). This set now carries the BESPOKE folder redrives only.
            // GTW-437: the injury hot-reload — rebuilds BOTH the InjuryRegistry and the
            // InjuryTables on an edit to ANY `injuries/**/*.injury.ron` OR `*.weighting.ron`
            // (one folder, two resources).
            redrive_injuries_on_asset_event,
            // GTW-489: the prefab hot-reload — rebuilds the UUID-keyed PrefabRegistry on a
            // `maps/**/*.prefab.ron` edit (NO edge-opening validation — the schema has none).
            redrive_prefabs_on_asset_event,
        ),
    );
}
