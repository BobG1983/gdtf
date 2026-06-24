//! GTW-257: `AppState::Load` preloads the `assets/weapons/` folder through the
//! `RonAsset<WeaponSpec>` loader (guarded for headless), builds a name-keyed
//! `WeaponRegistry` from the loaded weapon files (keyed by filename stem), and
//! gates the Load->Intro transition on it — so a battle never starts before weapons
//! load. This mirrors the theme's / situation's load-and-build path exactly.
//!
//! Two tiers (mirroring the theme / situation harness split):
//!
//! - **Tier (a)** — `MinimalPlugins` via [`GdtfTestAppBuilder`]: there is no
//!   `AssetServer`, so the weapons-loader registration + folder kick-off must no-op
//!   without panicking (the `asset_server.is_some()` guard, bevy-traps rule 1).
//!   Injecting a `GdtfTheme` + `CombatTuning` + `WeaponRegistry` drives the real
//!   gated transition; the app advances past `Load` and no registry is RESOLVED from
//!   disk (nothing to load).
//! - **Tier (b)** — `DefaultPlugins` (headless, `backends: None`) via
//!   [`GdtfLoadTestAppBuilder`]: a real `AssetServer` pointed at the workspace
//!   `assets/`. The good path loads `assets/weapons/*.ron` into a `WeaponRegistry`
//!   keyed by file stem (`stub_pistol`, `las_carbine`).
//!
//! These are *pin-discriminating*: each assertion re-encodes one acceptance
//! criterion so a regression turns the test red. They are VALUE-AGNOSTIC (only the
//! registry's PRESENCE + that it is keyed by the authored filename stems), so a
//! tuning edit never reddens them — the authored weapon magnitudes are tuning DATA,
//! NOT pinned by tests (the brittle-test rule; see
//! [`WeaponSpec`](gdtf_battle_sim::weapon::WeaponSpec)). The field-to-bundle
//! conversion MECHANISM is covered by the fixture-based sim round-trip
//! (`weapon::test::weapon_spec_round_trips_and_into_bundle_groups_faithfully`); this
//! harness proves the REAL `assets/weapons/` folder loads through the Load code path
//! and that its authored KEYS resolve.

use bevy::{app::Startup, state::state::State};
use gdtf_app::test_support::{AppState, LoadedSituation, seed_load_fallbacks};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::{WeaponName, WeaponRegistry},
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
/// on an async asset load resolving. The weapons folder load shares the
/// `AssetServer` with the theme / situation / tuning + the presenter's startup
/// tile-sheet loads (the full scene stack is registered in this harness), so under
/// parallel `cargo` contention the async resolve has NO fixed frame count. These
/// waits key off the resolved SIGNAL; the cap is a safety net against a genuine
/// never-resolve hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

/// AC4 (tier a) — under `MinimalPlugins` there is no `AssetServer`, so entering
/// `Load` must not panic: the weapons-loader registration (`init_ron_asset::<WeaponSpec>()`)
/// and the `load_folder("weapons")` kick-off both guard on a missing server and
/// no-op. The machine still advances past `Load` once a theme + tuning + registry
/// are injected (standing in for all resolves completing), proving the guard holds.
///
/// Pin: if the weapons-loader registration or the `load_folder` kick-off ever ran
/// without the `asset_server.is_some()` / `Option<Res<AssetServer>>` guard, entering
/// `Load` would panic instead of resting / advancing (bevy-traps rule 1).
#[test]
fn weapons_loader_no_ops_cleanly_without_asset_server() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Enter Load: the weapons loader registration + folder kick-off must no-op (no
    // AssetServer), not panic.
    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the kick-off must no-op and the machine rests in Load, not panic",
    );

    // Stand in for the theme + tuning + weapons + situation + armor resolves completing
    // (no AssetServer under MinimalPlugins), driving the real gated transition (GTW-257:
    // the WeaponRegistry is required before Load transitions; GTW-261: the
    // LoadedSituation too; GTW-269: the ArmorRegistry too).
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-384: the GangerStatTuning is a gate-blocking resource too.
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut().insert_resource(ArmorRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        TRANSITION_BUDGET,
    );
    assert!(
        reached_intro,
        "with a GdtfTheme + CombatTuning + WeaponRegistry + LoadedSituation + ArmorRegistry \
         present, Load must advance to Intro within {TRANSITION_BUDGET} updates; last observed \
         AppState was {:?}",
        app_state(&app),
    );
}

/// AC4 (companion, tier a) — the Load->Intro transition GATES on the
/// `WeaponRegistry`: with a theme + tuning present but NO registry (and no
/// `AssetServer` to resolve one), the machine must stay in `Load` for the whole
/// budget. Proves a battle can never start before weapons load.
///
/// Pin: this fails if the transition ever fired without the registry present
/// (regressing the GTW-257 gate clause), which would risk starting a battle
/// weapon-less.
#[test]
fn load_does_not_leave_without_a_weapon_registry() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Theme + tuning present, but the WeaponRegistry deliberately withheld.
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-384: the GangerStatTuning is a gate-blocking resource too.
    app.world_mut().insert_resource(GangerStatTuning::default());

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );

    assert!(
        !left_load,
        "Load must NOT leave while the WeaponRegistry is absent; it left to {:?}",
        app_state(&app),
    );
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no WeaponRegistry present, the machine stays in Load (a battle never starts \
         weapon-less)",
    );
}

/// AC4 (tier b) / GTW-270 AC — with a real `AssetServer` rooted at the workspace
/// `assets/`, entering `Load` loads `assets/weapons/*.weapon.ron` and builds a
/// `WeaponRegistry` keyed by each weapon's filename stem. Proves the folder loaded
/// into the registry keyed by filename (the canonical `stub_pistol` / `las_carbine`
/// keys resolve), and that the Load gate waited for it (the machine reaches Intro with
/// a registry present).
///
/// GTW-270: this asserts that the REAL weapons folder loads + each authored KEY
/// resolves to `Some(spec)` through the Load code path (mirroring the GTW-257
/// precedent). It does NOT pin any authored magnitude — those are tuning DATA (the
/// brittle-test rule); the field-to-bundle conversion mechanism is covered by the
/// fixture-based sim round-trip
/// (`weapon::test::weapon_spec_round_trips_and_into_bundle_groups_faithfully`).
#[test]
fn real_asset_resolves_weapon_registry_keyed_by_filename() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async weapons folder load: wait until the WeaponRegistry is
    // inserted, not a fixed frame count. Cap is a safety net (GTW-305).
    advance_until_resource_exists::<WeaponRegistry>(&mut app, LOAD_SAFETY_NET);

    // The registry is keyed by the authored filename stems, and each authored key
    // resolves to its loaded spec.
    if let Some(registry) = app.world().get_resource::<WeaponRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved WeaponRegistry must carry the authored (non-empty) weapons",
        );
        assert!(
            registry
                .spec(&WeaponName::new("stub_pistol".to_owned()))
                .is_some(),
            "the registry must hold the `stub_pistol` weapon (keyed by stub_pistol.weapon.ron's stem)",
        );
        assert!(
            registry
                .spec(&WeaponName::new("las_carbine".to_owned()))
                .is_some(),
            "the registry must hold the `las_carbine` weapon (keyed by las_carbine.weapon.ron's stem)",
        );
    }

    // The Load gate WAITED for the registry: the machine reaches Intro, and a
    // WeaponRegistry is present when it does (GTW-257 gate clause).
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once theme + tuning + weapons resolve; \
         last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<WeaponRegistry>().is_some(),
        "a WeaponRegistry must be present when Load reaches Intro (the gate waited for it)",
    );
    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the theme resolved alongside the weapons (the gated transition fired)",
    );
}

/// GTW-297 (`AC3b`) — the REAL auto-battle path: the `AutoBattlePlugin`'s
/// `seed_load_fallbacks` runs on `Startup` AND a real `AssetServer` is present
/// (the GUI auto-battle launch). The empty `WeaponRegistry::default()` seed must NOT
/// shadow the real folder resolve: with an `AssetServer` present the seed must NOT
/// insert an empty registry, so `poll_and_resolve` (which only runs `resolve_weapons`
/// while the registry is ABSENT) populates it from `assets/weapons/*.weapon.ron`.
///
/// This reproduces the bug's exact preconditions on the real code path: a
/// `GdtfLoadTestAppBuilder` app (live `AssetServer` rooted at the workspace `assets/`)
/// with the genuine `seed_load_fallbacks` system registered on `Startup`, exactly as
/// `AutoBattlePlugin::build` wires it. The assertions encode the fix:
///
/// - the resolved `WeaponRegistry` is non-empty and resolves the canonical `stub_pistol`
///   stem (so battle setup's `weapons.spec("stub_pistol")` would NOT return `None` /
///   `WeaponNotFound`);
/// - the machine reaches `Intro` with the registry present (the gate waited for the
///   REAL registry, not the empty seed).
///
/// PIN: if the seed reverts to inserting `WeaponRegistry::default()` UNCONDITIONALLY,
/// the registry is present from `Startup`, `poll_and_resolve` SKIPS `resolve_weapons`
/// (its `if !weapons_present` guard), the registry stays empty, `spec("stub_pistol")`
/// returns `None`, and this test goes red — exactly the `AC3b` black-screen failure.
#[test]
fn seeded_startup_does_not_shadow_real_weapon_resolution() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    // Wire the REAL auto-battle Startup seed (the path AutoBattlePlugin registers),
    // minus the unrelated `drive_past_menu` that needs the RunningState machinery.
    app.add_systems(Startup, seed_load_fallbacks);

    // Signal-poll the WeaponRegistry insert (not a fixed frame count): the real resolve
    // only inserts the registry ONCE it is fully built from the folder (it stays ABSENT
    // while members are still resolving — see `resolve_weapons`), so on the GOOD path
    // existence implies the authored weapons are present. The cap is a safety net
    // (GTW-305). The seed-shadow regression instead inserts an EMPTY registry at Startup,
    // which the assertions below catch immediately.
    advance_until_resource_exists::<WeaponRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<WeaponRegistry>() {
        // AC3b pin: with the AssetServer present the empty seed must NOT win — the real
        // folder resolve must populate a registry that holds the authored weapons.
        assert!(
            !registry.is_empty(),
            "the real folder resolve must populate the registry, not leave the empty seed",
        );
        assert!(
            registry
                .spec(&WeaponName::new("stub_pistol".to_owned()))
                .is_some(),
            "the registry must hold `stub_pistol` — the empty seed must NOT have shadowed the resolve",
        );
        assert!(
            registry
                .spec(&WeaponName::new("las_carbine".to_owned()))
                .is_some(),
            "the resolved registry must also hold `las_carbine` (the empty seed held neither)",
        );
    }

    // The gate waited for the REAL registry: the machine reaches Intro with it present,
    // so a real auto-battle run would advance toward BattleRunning rather than aborting
    // at Generation with WeaponNotFound.
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with the Startup seed present, Load must still reach Intro once the real weapons resolve; \
         last AppState was {:?}",
        app_state(&app),
    );
}
