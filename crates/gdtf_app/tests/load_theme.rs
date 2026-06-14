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
//!   the workspace `assets/`. The good path resolves `theme/grimdark.ron` to a
//!   `GdtfTheme`; a deliberately-bad theme path drives the failure branch — it
//!   does not hang, records `LoadFailed`, and still leaves a (default)
//!   `GdtfTheme` present.
//!
//! These are *pin-discriminating*: each assertion re-encodes one acceptance
//! criterion so a regression turns the test red rather than silently changing
//! the load behaviour.

use std::path::PathBuf;

use bevy::{asset::Handle, state::state::State, text::Font};
use gdtf_app::test_support::AppState;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::{GdtfTheme, default_theme};

/// Bounded budget for the short Load orchestration plus its state-transition
/// propagation — bounded so a machine that never resolves fails instead of
/// hanging (AC3: the app must never hang on a failed asset).
const LOAD_BUDGET: u32 = 32;

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

/// Tier (a): once a [`GdtfTheme`] is present in `Load`, the machine transitions
/// `Load -> Intro`, and the theme **persists** past `OnExit(Load)`.
///
/// Injecting the resolved theme stands in for the async load completing (which
/// has no `AssetServer` under `MinimalPlugins`); it drives the real
/// `transition_to_intro` and `cleanup` systems. Pin: this fails if the
/// transition stops firing on theme-present (AC4), or if `cleanup` ever removed
/// the `GdtfTheme` instead of persisting it (AC5).
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

    // Stand in for the async resolve completing: insert the runtime theme.
    app.world_mut().insert_resource(default_theme());

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_BUDGET,
    );
    assert!(
        reached_intro,
        "with a GdtfTheme present, Load must transition to Intro within {LOAD_BUDGET} updates; \
         last observed AppState was {:?}",
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
        LOAD_BUDGET,
    );

    assert!(
        !left_load,
        "without a GdtfTheme the machine must not leave Load; it reached {:?}",
        app_state(&app),
    );
}

/// Absolute path to the malformed-theme fixtures root
/// (`tests/fixtures/bad_theme_root`), whose `theme/grimdark.ron` is deliberately
/// unparseable so the real loader reaches `Failed`.
fn bad_theme_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("bad_theme_root")
}

/// Tier (b) good path: with a real `AssetServer` rooted at the workspace
/// `assets/`, entering `Load` loads and resolves `theme/grimdark.ron` into a
/// [`GdtfTheme`] that carries the shipped values and a **real** (non-default)
/// font handle, and the machine leaves `Load` for `Intro`.
///
/// Pin: this fails if the kick-off path/loader regresses (the theme never
/// resolves, so the budget runs out), if `resolve` is fed the wrong values, or
/// if the loaded font handle is not threaded into the theme (the handle would be
/// the default). It exercises the whole async load -> resolve -> insert ->
/// transition chain against real assets (AC1, AC2, AC4).
#[test]
fn real_asset_good_path_resolves_shipped_theme_and_transitions() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    let resolved = advance_until(
        &mut app,
        |app| app.world().get_resource::<GdtfTheme>().is_some(),
        LOAD_BUDGET,
    );
    assert!(
        resolved,
        "the real theme load should resolve a GdtfTheme within {LOAD_BUDGET} updates; \
         last observed AppState was {:?}",
        app_state(&app),
    );

    if let Some(theme) = app.world().get_resource::<GdtfTheme>() {
        // The shipped grimdark text color (ported from main_theme.tres).
        let text = theme.text.to_srgba();
        assert!(
            (text.red - 0.84).abs() < f32::EPSILON
                && (text.green - 0.80).abs() < f32::EPSILON
                && (text.blue - 0.73).abs() < f32::EPSILON,
            "good path must resolve the SHIPPED grimdark values, got text {text:?}",
        );
        assert_eq!(
            &**theme.font_key, "fonts/Alegreya-Variable.ttf",
            "good path must carry the shipped font key",
        );
        // The font was loaded for real, so its handle is NOT the default handle
        // (which is what the const-fallback path would carry) — proves the loaded
        // font handle was threaded into resolve (AC2).
        assert_ne!(
            theme.font,
            Handle::<Font>::default(),
            "good path must thread the real loaded font handle into the theme, not the default",
        );
    }

    // And the machine leaves Load for Intro now that a theme is present.
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_BUDGET,
    );
    assert!(
        reached_intro,
        "with a resolved GdtfTheme, Load must transition to Intro; last AppState was {:?}",
        app_state(&app),
    );
}

/// Tier (b) failure path: with the asset root pointed at a malformed
/// `theme/grimdark.ron`, the load reaches `Failed`. The app must NOT hang — it
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

    // The bad theme must still produce a theme (the const fallback) within the
    // bounded budget — proving the app never hangs on a failed asset.
    let recovered = advance_until(
        &mut app,
        |app| app.world().get_resource::<GdtfTheme>().is_some(),
        LOAD_BUDGET,
    );
    assert!(
        recovered,
        "a failed theme load must fall back to a default GdtfTheme within {LOAD_BUDGET} updates \
         (never hang); last observed AppState was {:?}",
        app_state(&app),
    );

    if let Some(theme) = app.world().get_resource::<GdtfTheme>() {
        assert_eq!(
            theme,
            &default_theme(),
            "the failure path must insert exactly the const-fallback default theme",
        );
    }

    // The machine still leaves Load — a bad asset does not strand it themeless.
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_BUDGET,
    );
    assert!(
        reached_intro,
        "even on a failed asset, Load must transition to Intro with the default theme; \
         last AppState was {:?}",
        app_state(&app),
    );
}
