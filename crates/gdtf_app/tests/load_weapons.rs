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
//!   keyed by file stem (`autogun`, `lasgun`).
//!
//! These are *pin-discriminating*: each assertion re-encodes one acceptance
//! criterion so a regression turns the test red. VALUE-AGNOSTIC throughout — no
//! pinned weapon magnitude is ever asserted (only the registry's PRESENCE + that it
//! is keyed by the authored filename stems), so a tuning edit to a weapon `.ron`
//! never reddens these tests.

use bevy::state::state::State;
use gdtf_app::test_support::{AppState, LoadedSituation};
use gdtf_battle_sim::{
    situation::Situation,
    tuning::CombatTuning,
    weapon::{WeaponName, WeaponRegistry},
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::{GdtfTheme, default_theme};

/// Bounded budget for the Load orchestration plus its state-transition propagation.
/// Sized generously: the weapons folder load shares the `AssetServer` with the
/// theme/situation/tuning + the presenter's startup tile-sheet loads (the full scene
/// stack is registered in this harness), so the async resolve can need many headless
/// `update()` polls under contention (the `load_situation.rs` budget).
const LOAD_BUDGET: u32 = 512;

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

    // Stand in for the theme + tuning + weapons + situation resolves completing (no
    // AssetServer under MinimalPlugins), driving the real gated transition (GTW-257:
    // the WeaponRegistry is required before Load transitions; GTW-261: the
    // LoadedSituation too).
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation(Situation::default()));

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_BUDGET,
    );
    assert!(
        reached_intro,
        "with a GdtfTheme + CombatTuning + WeaponRegistry + LoadedSituation present, Load must \
         advance to Intro within {LOAD_BUDGET} updates; last observed AppState was {:?}",
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

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        LOAD_BUDGET,
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

/// AC4 (tier b) — with a real `AssetServer` rooted at the workspace `assets/`,
/// entering `Load` loads `assets/weapons/*.ron` and builds a `WeaponRegistry` keyed
/// by each weapon's filename stem. Proves the folder loaded into the registry keyed
/// by filename (the authored `autogun` / `lasgun` keys resolve), and that the Load
/// gate waited for it (the machine reaches Intro with a registry present).
///
/// Value-agnostic: it asserts the registry is PRESENT, non-empty, and resolves the
/// authored STEMS — never a pinned weapon magnitude.
#[test]
fn real_asset_resolves_weapon_registry_keyed_by_filename() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // The real weapons folder load resolves a WeaponRegistry within the budget.
    let resolved = advance_until(
        &mut app,
        |app| app.world().get_resource::<WeaponRegistry>().is_some(),
        LOAD_BUDGET,
    );
    assert!(
        resolved,
        "the real weapons folder load should resolve a WeaponRegistry within {LOAD_BUDGET} \
         updates; last observed AppState was {:?}",
        app_state(&app),
    );

    // The registry is keyed by the authored filename stems (presence, not a value).
    if let Some(registry) = app.world().get_resource::<WeaponRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved WeaponRegistry must carry the authored (non-empty) weapons",
        );
        assert!(
            registry
                .spec(&WeaponName::new("autogun".to_owned()))
                .is_some(),
            "the registry must hold the `autogun` weapon (keyed by autogun.ron's stem)",
        );
        assert!(
            registry
                .spec(&WeaponName::new("lasgun".to_owned()))
                .is_some(),
            "the registry must hold the `lasgun` weapon (keyed by lasgun.ron's stem)",
        );
    }

    // The Load gate WAITED for the registry: the machine reaches Intro, and a
    // WeaponRegistry is present when it does (GTW-257 gate clause).
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_BUDGET,
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
