//! GTW-143: `AppState::Load` loads the theme + font, resolves the runtime
//! [`GdtfTheme`], inserts it before leaving `Load`, then transitions to `Intro`.
//!
//! Two tiers (per the ticket's headless test strategy):
//!
//! - **Tier (a)** — `MinimalPlugins` via [`GdtfTestAppBuilder`]: there is no
//!   `AssetServer`, so the kick-off must no-op without panicking; injecting a
//!   `GdtfTheme` while in `Load` drives the real `transition_to_intro` /
//!   `cleanup` path, proving the theme is present before exit, persists past it,
//!   and the machine moves `Load -> Intro`.
//! - **Tier (b)** — `DefaultPlugins` (headless, `backends: None`) via
//!   `gdtf_test_utils::GdtfLoadTestAppBuilder`: a real `AssetServer` pointed at
//!   the workspace `assets/`. The good path resolves `core_tuning/ui_theme.tuning.ron` to a
//!   `GdtfTheme`; a deliberately-bad theme path drives the failure branch — it
//!   does not hang, records `LoadFailed`, and still leaves a (default)
//!   `GdtfTheme` present.
//!
//! These are *pin-discriminating*: each assertion re-encodes one acceptance
//! criterion so a regression turns the test red rather than silently changing
//! the load behaviour.

use std::path::PathBuf;

use bevy::{asset::Handle, state::state::State, text::Font};
use gdtf_app::test_support::{AppState, LoadedSituation};
use gdtf_battle_sim::{
    injuries::InjuryRegistry,
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::WeaponRegistry,
};
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_ui::theme::{GdtfTheme, default_theme};

/// Bounded budget for the short Load orchestration plus its state-transition
/// propagation in the Tier (a) `MinimalPlugins` tests, where all gate resources
/// are injected by hand: with nothing left to load this is a true, small,
/// deterministic frame count (no async load to wait on).
const TRANSITION_BUDGET: u32 = 32;

/// Generous SAFETY-NET cap for the real-asset (Tier b) `advance_until` waits that
/// are gated on an async asset load resolving (the `Load -> Intro` transition
/// needs theme + tuning + weapons + situation all resolved). It is a safety net
/// against a genuine never-resolve hang, NOT a timing budget: an async load polled
/// under parallel `cargo` contention has no fixed frame count, so the wait keys off
/// the resolved SIGNAL and merely caps the worst case high enough to absorb any
/// variance (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

/// Tier (a): under `MinimalPlugins` there is no `AssetServer`, so entering
/// `Load` must not panic — the kick-off guards on a missing server and no-ops.
///
/// Pin: if `kick_off_loads` ever took a bare `Res<AssetServer>` (or otherwise
/// assumed the server exists), this update would panic instead of resting in
/// `Load` (bevy-traps rule 1). With no server no `GdtfTheme` is produced, so the
/// machine stays in `Load`.
#[test]
fn entering_load_without_asset_server_does_not_panic() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    app.update();

    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the kick-off must no-op and the machine rests in Load, not panic",
    );
    assert!(
        app.world().get_resource::<GdtfTheme>().is_none(),
        "no AssetServer means no load resolves, so no GdtfTheme should be inserted",
    );
}

/// Tier (a): once a [`GdtfTheme`] (and the GTW-206 [`CombatTuning`]) are present
/// in `Load`, the machine transitions `Load -> Intro`, and the theme **persists**
/// past `OnExit(Load)`.
///
/// Injecting the resolved theme + tuning stands in for the async loads completing
/// (which have no `AssetServer` under `MinimalPlugins`); it drives the real
/// `transition_to_intro` and `cleanup` systems. Pin: this fails if the transition
/// stops firing on theme+tuning-present (AC4 / GTW-206 AC5), or if `cleanup` ever
/// removed the `GdtfTheme` instead of persisting it (AC5).
#[test]
fn theme_present_transitions_to_intro_and_persists() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Enter Load (kick-off no-ops: no AssetServer).
    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "precondition: rest in Load"
    );

    // Stand in for the async resolves completing: insert the runtime theme + the
    // GTW-206 tuning + the GTW-257 WeaponRegistry + the GTW-261 LoadedSituation (all
    // required before Load transitions).
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-384: the Load gate also requires a GangerStatTuning; default clears it.
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-505: the Load->Intro gate also requires a MeleeWeaponRegistry (empty-default
    // seed stands in for the asset-less resolve, mirroring the WeaponRegistry seed above).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    // GTW-269: the Load gate also requires an ArmorRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load gate also requires a GangRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-489: the gate-blocking UUID-keyed PrefabRegistry2; empty clears it (GTW-494 retired
    // the legacy prefab-registry gate).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry2::default());
    // GTW-487: the gate-blocking UUID-keyed TerrainDefRegistry + UuidThemeRegistry (GTW-494
    // retired the legacy terrain / theme registry gates).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        TRANSITION_BUDGET,
    );
    assert!(
        reached_intro,
        "with a GdtfTheme present, Load must transition to Intro within {TRANSITION_BUDGET} \
         updates; last observed AppState was {:?}",
        app_state(&app),
    );

    // The resolved theme is the deliberate exception that survives OnExit(Load).
    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the GdtfTheme must persist past OnExit(Load) — it is the state-scoped-resource exception",
    );
}

/// Tier (a) — explicit guard on `Load` not auto-leaving while themeless: with no
/// `GdtfTheme` ever inserted (no `AssetServer` to resolve one), the machine must
/// stay in `Load` for the whole budget rather than transitioning to `Intro`.
///
/// Pin: this fails if the transition ever fires without a theme present (AC4:
/// the machine must NOT leave `Load` without a `GdtfTheme`). Combined with the
/// previous test, it brackets the transition condition from both sides.
#[test]
fn load_does_not_leave_without_a_theme() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );

    assert!(
        !left_load,
        "without a GdtfTheme the machine must not leave Load; it reached {:?}",
        app_state(&app),
    );
}

/// Absolute path to the malformed-theme fixtures root
/// (`tests/fixtures/bad_theme_root`), whose `core_tuning/ui_theme.tuning.ron` is deliberately
/// unparseable so the real loader reaches `Failed`.
fn bad_theme_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("bad_theme_root")
}

/// Tier (b) good path: with a real `AssetServer` rooted at the workspace
/// `assets/`, entering `Load` loads and resolves `core_tuning/ui_theme.tuning.ron` (and
/// preloads the fonts folder) into a [`GdtfTheme`] whose text-bearing sub-themes
/// carry **real** (non-default) font handles, and the machine leaves `Load` for
/// `Intro`.
///
/// Pin: this fails if the kick-off path/loader regresses (the theme never
/// resolves, so the budget runs out), or if the loaded font handles are not
/// threaded into the theme (they would be the default). It exercises the whole
/// async load -> resolve -> insert -> transition chain against real assets.
/// Value-agnostic per the de-brittle principle (GTW-148): it asserts the
/// resolution mechanism, not the tunable shipped colors.
#[test]
fn real_asset_good_path_resolves_shipped_theme_and_transitions() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async theme load: wait until the resolved GdtfTheme is
    // inserted (covers both the success-resolve and the failure-default paths),
    // not a fixed frame count — the cap is a safety net (GTW-305).
    advance_until_resource_exists::<GdtfTheme>(&mut app, LOAD_SAFETY_NET);

    if let Some(theme) = app.world().get_resource::<GdtfTheme>() {
        assert_eq!(
            &**theme.default_font, "fonts/Alegreya-Variable.ttf",
            "good path must carry the shipped default_font key",
        );
        // The fonts were loaded for real, so each text-bearing sub-theme's handle
        // is NOT the default handle (what the const-fallback path would carry) —
        // proving the loaded font handles were threaded into resolve.
        for (label, font) in [
            ("button", theme.button.font.clone()),
            ("title", theme.title.font.clone()),
            ("text", theme.text.font.clone()),
        ] {
            assert_ne!(
                font,
                Handle::<Font>::default(),
                "good path must thread the real loaded {label} font handle into the theme",
            );
        }
    }

    // And the machine leaves Load for Intro once ALL gate resources resolve. That
    // transition is itself gated on the remaining async loads (tuning / weapons /
    // situation), so it gets the generous safety-net cap, not a frame budget.
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a resolved GdtfTheme, Load must transition to Intro; last AppState was {:?}",
        app_state(&app),
    );
}

/// Tier (b) multi-font: the fonts folder preload resolves DISTINCT font handles —
/// the title sub-theme's overriding `Cinzel-Variable.ttf` is loaded distinct from
/// the button / text sub-themes' default `Alegreya-Variable.ttf` (GTW-149).
///
/// Pin: this fails if the per-sub-theme font override is dropped (all three would
/// share the default handle), or if the folder preload regresses so only the
/// default font is resident. The harness roots at the workspace `assets/`, which
/// ships both fonts.
#[test]
fn real_asset_multi_font_load_resolves_distinct_title_font() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async multi-font theme load (cap is a safety net, GTW-305).
    advance_until_resource_exists::<GdtfTheme>(&mut app, LOAD_SAFETY_NET);

    if let Some(theme) = app.world().get_resource::<GdtfTheme>() {
        // Title overrides its font; button + text fall to the default_font. The
        // overriding font handle must differ from the default one — proving the
        // Cinzel override loaded distinct from Alegreya across the folder preload.
        assert_ne!(
            theme.title.font, theme.button.font,
            "title's overriding font must resolve to a DISTINCT handle from the default font",
        );
        assert_eq!(
            theme.button.font, theme.text.font,
            "button + text both use the default_font, so they share one handle",
        );
        assert_ne!(
            theme.title.font,
            Handle::<Font>::default(),
            "the override font must be a real loaded handle, not the default",
        );
    }
}

/// Tier (b) failure path: with the asset root pointed at a malformed
/// `core_tuning/ui_theme.tuning.ron`, the load reaches `Failed`. The app must NOT hang — it
/// records the failure, falls back to the const-default [`GdtfTheme`], and leaves
/// `Load` within the bounded budget.
///
/// Pin: this fails if a failed asset hangs the machine (the budget runs out
/// before a theme appears), or if the failure path does not insert the
/// const-fallback theme (AC3). The fallback equals [`default_theme`], so this
/// asserts the resolved resource is exactly that, distinguishing the failure
/// path from the good path.
#[test]
fn real_asset_failure_path_does_not_hang_and_uses_default_theme() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(bad_theme_root())
        .starting_in(AppState::Load)
        .build();

    // The bad theme must still produce a theme (the const fallback): the failure
    // path inserts the default, so the same inserted-resource signal fires — proving
    // the app never hangs on a failed asset. Signal-poll, cap is a safety net.
    advance_until_resource_exists::<GdtfTheme>(&mut app, LOAD_SAFETY_NET);

    if let Some(theme) = app.world().get_resource::<GdtfTheme>() {
        assert_eq!(
            theme,
            &default_theme(),
            "the failure path must insert exactly the const-fallback default theme",
        );
    }

    // The machine still leaves Load — a bad asset does not strand it themeless. The
    // transition is gated on the remaining async loads, so it gets the safety-net cap.
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "even on a failed asset, Load must transition to Intro with the default theme; \
         last AppState was {:?}",
        app_state(&app),
    );
}
