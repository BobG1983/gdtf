//! GTW-494 (child T08 of the GTW-476 refactor): `AppState::Load` preloads the per-theme
//! `assets/terrain/<theme>/` folder through the GTW-487 `RonAsset<TerrainDef>` loader
//! (guarded for headless), builds the UUID-keyed [`TerrainDefRegistry`] from the loaded
//! `*.terrain_def.ron` files (keyed by each def's OWN UUID), and gates the Load→Intro
//! transition on it — so the machine never leaves `Load` before the per-theme terrain
//! folder is verified loaded.
//!
//! This file was MIGRATED off the retired flat-dir `resolve_terrain` per-file terrain model
//! (GTW-394) onto the UUID model: GTW-494 removed the old game-side loader, so the
//! [`TerrainDefRegistry`] is now the ONLY terrain resolver in the Load flow (the sim +
//! procgen + presenter consume it as of GTW-491/492/493).
//!
//! Two tiers:
//!
//! - **Tier (a)** — `MinimalPlugins` via [`GdtfTestAppBuilder`]: there is no `AssetServer`,
//!   so the loader registration + folder kick-off must no-op without panicking (the
//!   `asset_server.is_some()` guard, bevy-traps rule 1). Injecting all gate resources drives
//!   the real gated transition; a companion test withholds the new terrain registry to prove
//!   it is a genuine gate-blocking resource.
//! - **Tier (b)** — `DefaultPlugins` (headless, `backends: None`) via
//!   [`GdtfLoadTestAppBuilder`]: a real `AssetServer` pointed at the workspace `assets/`. The
//!   good path loads `assets/terrain/<theme>/*.terrain_def.ron` into a [`TerrainDefRegistry`]
//!   keyed by each def's OWN UUID, and a KNOWN authored UUID resolves.
//!
//! VALUE-AGNOSTIC (gate 4a): asserts presence / known-UUID resolution / gate-blocking ONLY —
//! no authored terrain magnitudes pinned.

use gdtf_app::test_support::{AppState, LoadedSituation};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    injuries::InjuryRegistry,
    level::UuidThemeRegistry,
    procgen::ProcgenTuning,
    situation::Situation,
    terrain::def::{TerrainDefRegistry, TerrainUuid},
    tuning::{CombatTuning, GangerStatTuning},
    weapon::WeaponRegistry,
};
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_ui::theme::{GdtfTheme, default_theme};

/// Bounded budget for the Tier (a) `MinimalPlugins` transition / negative waits,
/// where all gate resources are injected by hand — a true, small, deterministic
/// frame count (no async load to wait on).
const TRANSITION_BUDGET: u32 = 32;

/// Generous SAFETY-NET cap for the real-asset (Tier b) `advance_until` waits gated
/// on an async asset load resolving. The per-theme `terrain/` folder load shares the
/// `AssetServer` with the theme / situation / tuning / weapons / armor + the presenter's
/// startup tile-sheet loads (the full scene stack is registered in this harness), so under
/// parallel `cargo` contention the async resolve has NO fixed frame count. These waits key
/// off the resolved SIGNAL; the cap is a safety net against a genuine never-resolve hang, not
/// a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The migrated `industrial_hive` `deck_floor` [`TerrainUuid`] (`Uuid::from_u128(0x0184_0a91_0004)`).
const fn deck_floor_uuid() -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a91_0004))
}

/// The migrated `industrial_hive` `bulkhead_wall` [`TerrainUuid`]
/// (`Uuid::from_u128(0x0184_0a91_0002)`).
const fn bulkhead_wall_uuid() -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a91_0002))
}

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world()
        .resource::<bevy::state::state::State<AppState>>()
        .get()
        .clone()
}

/// Seeds every gate-blocking resource EXCEPT the one the caller withholds via `seed_terrain`
/// — the tier (a) harness helper standing in for all resolves completing under
/// `MinimalPlugins` (no `AssetServer`).
fn seed_gate_resources(app: &mut bevy::app::App, seed_terrain: bool) {
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(GangerStatTuning::default());
    // GTW-533: the ProcgenTuning gate-blocking resource (the Generation procgen trigger
    // reads it) — seed it alongside the others to reach Intro.
    app.world_mut().insert_resource(ProcgenTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-505: the Load->Intro gate also requires a MeleeWeaponRegistry (empty-default
    // seed stands in for the asset-less resolve, mirroring the WeaponRegistry seed above).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::AttachmentRegistry::default());
    app.world_mut().insert_resource(ArmorRegistry::default());
    // GTW-545: the FieldDefRegistry is a gate-blocking resource too; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::FieldDefRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-489: the gate-blocking UUID-keyed PrefabRegistry.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    // GTW-487: the gate-blocking UUID-keyed UuidThemeRegistry.
    app.world_mut()
        .insert_resource(UuidThemeRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));
    if seed_terrain {
        // GTW-487 / GTW-494: the gate-blocking UUID-keyed TerrainDefRegistry.
        app.world_mut()
            .insert_resource(TerrainDefRegistry::default());
    }
}

/// AC (tier a) — under `MinimalPlugins` there is no `AssetServer`, so entering `Load` must
/// not panic: the loader registration (`init_ron_asset_with_extensions::<TerrainDef>`) and
/// the `load_folder("terrain")` kick-off both guard on a missing server and no-op. The
/// machine still advances past `Load` once all gate resources are injected (standing in for
/// all resolves completing), proving the guard holds.
///
/// Pin: if the loader registration or the `load_folder` kick-off ever ran without the
/// `asset_server.is_some()` / `Option<Res<AssetServer>>` guard, entering `Load` would panic
/// instead of resting / advancing (bevy-traps rule 1).
#[test]
fn terrain_loader_no_ops_cleanly_without_asset_server() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Enter Load: the terrain loader registration + folder kick-off must no-op (no
    // AssetServer), not panic.
    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the kick-off must no-op and the machine rests in Load, not panic",
    );

    // Stand in for every resolve completing (no AssetServer under MinimalPlugins), driving the
    // real gated transition (GTW-487: the TerrainDefRegistry is required before Load exits).
    seed_gate_resources(&mut app, true);

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        TRANSITION_BUDGET,
    );
    assert!(
        reached_intro,
        "with every gate resource (incl. the UUID-keyed TerrainDefRegistry) present, Load must \
         advance to Intro within {TRANSITION_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );
}

/// AC (companion, tier a) — the Load→Intro transition GATES on the [`TerrainDefRegistry`]:
/// with all other gate resources present but NO terrain registry (and no `AssetServer` to
/// resolve one), the machine must stay in `Load` for the whole budget. Proves the UUID-keyed
/// terrain registry is a genuine gate-blocking resource.
///
/// Pin: this fails if the transition ever fired without the terrain registry present
/// (regressing the GTW-487 gate clause), which would let `Load` exit before the per-theme
/// terrain folder resolved.
#[test]
fn load_does_not_leave_without_a_terrain_def_registry() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // All other gate resources present, but the TerrainDefRegistry deliberately withheld.
    seed_gate_resources(&mut app, false);

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );

    assert!(
        !left_load,
        "Load must NOT leave while the TerrainDefRegistry is absent; it left to {:?}",
        app_state(&app),
    );
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no TerrainDefRegistry present, the machine stays in Load (the per-theme terrain \
         folder must be verified loaded before Load exits)",
    );
}

/// AC (tier b) / GTW-494 C2 — with a real `AssetServer` rooted at the workspace `assets/`,
/// entering `Load` loads `assets/terrain/<theme>/*.terrain_def.ron` and builds a
/// [`TerrainDefRegistry`] keyed by each def's OWN UUID through the ACTUAL `resolve_terrain_defs`
/// branch. Proves the folder loaded into the registry (non-empty + known authored UUIDs
/// resolve), and that the Load gate waited for it (the machine reaches Intro with a registry
/// present).
///
/// Does NOT seed [`TerrainDefRegistry::default()`] — seeding would mask the very regression
/// under test. Existence of the resource here proves `resolve_terrain_defs` published it from
/// the real folder (not a hand-seeded default). VALUE-AGNOSTIC: presence + known-UUID
/// resolution only.
#[test]
fn real_asset_resolves_terrain_def_registry_by_uuid() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async per-theme terrain folder load: wait until the TerrainDefRegistry
    // is inserted, not a fixed frame count. Cap is a safety net (GTW-305).
    advance_until_resource_exists::<TerrainDefRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved TerrainDefRegistry must carry the migrated (non-empty) terrain defs",
        );
        assert!(
            registry.def(&deck_floor_uuid()).is_some(),
            "the registry must resolve the known authored `deck_floor` TerrainUuid (keyed by the \
             def's own UUID, not the filename) — the C2 known-UUID-resolves contract",
        );
        assert!(
            registry.def(&bulkhead_wall_uuid()).is_some(),
            "the registry must resolve the known authored `bulkhead_wall` TerrainUuid",
        );
    }

    // The Load gate WAITED for the registry: the machine reaches Intro, and a
    // TerrainDefRegistry is present when it does (GTW-487 gate clause).
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once every folder (incl. the per-theme \
         terrain/) resolves; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<TerrainDefRegistry>().is_some(),
        "a TerrainDefRegistry must be present when Load reaches Intro (the gate waited for it)",
    );
    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the theme resolved alongside the terrain (the gated transition fired)",
    );
}
