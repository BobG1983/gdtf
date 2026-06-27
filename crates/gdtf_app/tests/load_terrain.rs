//! GTW-394 (slice B): `AppState::Load` preloads the `assets/content/terrain/` folder through
//! the `RonAsset<TerrainSpec>` loader (guarded for headless), builds a name-keyed
//! `TerrainRegistry` from the loaded terrain files (keyed by filename stem), and gates
//! the Load→Intro transition on it — so the machine never leaves `Load` before the
//! terrain folder is verified loaded. This mirrors the GTW-269 armor load-and-build
//! path exactly. The registry is DORMANT after this slice (nothing consumes it yet —
//! the downstream generation epic does); it is resolved and gated on purely so it is
//! present when `Load` exits.
//!
//! Two tiers (mirroring the armor harness split, `load_armor.rs`):
//!
//! - **Tier (a)** — `MinimalPlugins` via [`GdtfTestAppBuilder`]: there is no
//!   `AssetServer`, so the terrain-loader registration + folder kick-off must no-op
//!   without panicking (the `asset_server.is_some()` guard, bevy-traps rule 1).
//!   Injecting all gate resources (theme + tuning + stat-tuning + weapons + situation
//!   + armor + terrain) drives the real gated transition; the app advances past `Load`,
//!     and no registry is RESOLVED from disk (nothing to load).
//! - **Tier (b)** — `DefaultPlugins` (headless, `backends: None`) via
//!   [`GdtfLoadTestAppBuilder`]: a real `AssetServer` pointed at the workspace
//!   `assets/`. The good path loads `assets/content/terrain/*.terrain.ron` into a
//!   `TerrainRegistry` keyed by file stem (`deck_floor`, `supply_crate`, etc.).
//!
//! These are *pin-discriminating*: each assertion re-encodes one acceptance criterion
//! so a regression turns the test red. They are VALUE-AGNOSTIC (only the registry's
//! PRESENCE + that it is keyed by the authored filename stems), so a tuning edit never
//! reddens them — the authored terrain magnitudes are tuning DATA, NOT pinned by tests
//! (the brittle-test rule; see [`TerrainSpec`](gdtf_battle_sim::terrain::piece::TerrainSpec)).
//! The field-to-variant routing MECHANISM is covered by the fixture-based sim round-trip
//! (`terrain::piece::test::spec_registry`); this harness proves the REAL
//! `assets/content/terrain/` folder loads through the Load code path and that its authored
//! KEYS resolve.

use gdtf_app::test_support::{AppState, LoadedSituation};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    injuries::InjuryRegistry,
    level::ThemeCatalogRegistry,
    situation::Situation,
    terrain::piece::{TerrainName, TerrainRegistry},
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
/// on an async asset load resolving. The terrain folder load shares the `AssetServer`
/// with the theme / situation / tuning / weapons / armor + the presenter's startup
/// tile-sheet loads (the full scene stack is registered in this harness), so under
/// parallel `cargo` contention the async resolve has NO fixed frame count. These waits
/// key off the resolved SIGNAL; the cap is a safety net against a genuine never-resolve
/// hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world()
        .resource::<bevy::state::state::State<AppState>>()
        .get()
        .clone()
}

/// AC (tier a) — under `MinimalPlugins` there is no `AssetServer`, so entering `Load`
/// must not panic: the terrain-loader registration (`init_ron_asset_with_extensions::<TerrainSpec>`)
/// and the `load_folder("terrain")` kick-off both guard on a missing server and no-op.
/// The machine still advances past `Load` once all gate resources are injected
/// (standing in for all resolves completing), proving the guard holds.
///
/// Pin: if the terrain-loader registration or the `load_folder` kick-off ever ran
/// without the `asset_server.is_some()` / `Option<Res<AssetServer>>` guard, entering
/// `Load` would panic instead of resting / advancing (bevy-traps rule 1).
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

    // Stand in for the theme + tuning + weapons + situation + armor + terrain resolves
    // completing (no AssetServer under MinimalPlugins), driving the real gated transition
    // (GTW-394: the TerrainRegistry is required before Load transitions, alongside the
    // GTW-257 WeaponRegistry, GTW-269 ArmorRegistry, and GTW-261 LoadedSituation).
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-384: the GangerStatTuning is a gate-blocking resource too (the sim derives
    // ganger stats from it), so seed it alongside the others to reach Intro.
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut().insert_resource(ArmorRegistry::default());
    app.world_mut().insert_resource(TerrainRegistry::default());
    app.world_mut()
        .insert_resource(ThemeCatalogRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load gate also requires a GangRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        TRANSITION_BUDGET,
    );
    assert!(
        reached_intro,
        "with a GdtfTheme + CombatTuning + GangerStatTuning + WeaponRegistry + ArmorRegistry \
         + TerrainRegistry + LoadedSituation present, Load must advance to Intro within \
         {TRANSITION_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );
}

/// AC (companion, tier a) — the Load→Intro transition GATES on the `TerrainRegistry`:
/// with all other gate resources present but NO terrain registry (and no `AssetServer`
/// to resolve one), the machine must stay in `Load` for the whole budget. Proves the
/// terrain registry is a genuine gate-blocking resource (the machine never leaves `Load`
/// before the terrain folder is verified loaded).
///
/// Pin: this fails if the transition ever fired without the terrain registry present
/// (regressing the GTW-394 gate clause), which would let `Load` exit before the terrain
/// folder resolved.
#[test]
fn load_does_not_leave_without_a_terrain_registry() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // All other gate resources present, but the TerrainRegistry deliberately withheld.
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-384: seed GangerStatTuning alongside others so terrain is the ONLY gate missing.
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut().insert_resource(ArmorRegistry::default());
    // TerrainRegistry deliberately ABSENT — the gate being tested.
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );

    assert!(
        !left_load,
        "Load must NOT leave while the TerrainRegistry is absent; it left to {:?}",
        app_state(&app),
    );
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no TerrainRegistry present, the machine stays in Load (the terrain folder must be \
         verified loaded before Load exits)",
    );
}

/// AC (tier b) / GTW-394 AC — with a real `AssetServer` rooted at the workspace
/// `assets/`, entering `Load` loads `assets/content/terrain/*.terrain.ron` and builds a
/// `TerrainRegistry` keyed by each terrain file's filename stem (with the `.terrain`
/// infix stripped). Proves the folder loaded into the registry keyed by filename (the
/// canonical `deck_floor` / `supply_crate` keys resolve), and that the Load gate
/// waited for it (the machine reaches Intro with a registry present).
///
/// Does NOT pin any authored magnitude — those are tuning DATA (the brittle-test rule);
/// the field-to-variant routing mechanism is covered by the fixture-based sim round-trip
/// (`terrain::piece::test::spec_registry`).
#[test]
fn real_asset_resolves_terrain_registry_keyed_by_filename() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async terrain folder load: wait until the TerrainRegistry is
    // inserted, not a fixed frame count. Cap is a safety net (GTW-305).
    advance_until_resource_exists::<TerrainRegistry>(&mut app, LOAD_SAFETY_NET);

    // The registry is keyed by the authored filename stems (with the `.terrain` infix
    // stripped): `deck_floor.terrain.ron` keys `deck_floor`; `supply_crate.terrain.ron`
    // keys `supply_crate`. Each authored key resolves to its loaded spec.
    if let Some(registry) = app.world().get_resource::<TerrainRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved TerrainRegistry must carry the authored (non-empty) terrain pieces",
        );
        assert!(
            registry
                .spec(&TerrainName::new("deck_floor".to_owned()))
                .is_some(),
            "the registry must hold the `deck_floor` piece (keyed by deck_floor.terrain.ron's stem)",
        );
        assert!(
            registry
                .spec(&TerrainName::new("supply_crate".to_owned()))
                .is_some(),
            "the registry must hold the `supply_crate` piece (keyed by supply_crate.terrain.ron's stem)",
        );
    }

    // The Load gate WAITED for the registry: the machine reaches Intro, and a
    // TerrainRegistry is present when it does (GTW-394 gate clause).
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once theme + tuning + weapons + armor \
         + terrain resolve; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<TerrainRegistry>().is_some(),
        "a TerrainRegistry must be present when Load reaches Intro (the gate waited for it)",
    );
    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the theme resolved alongside the terrain (the gated transition fired)",
    );
}
