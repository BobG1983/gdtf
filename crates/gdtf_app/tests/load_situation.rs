//! GTW-205 (E10.3): `AppState::Load` loads the authored `Situation` `.ron`
//! through the `RonAsset<Situation>` loader, guarded for headless, and resolves
//! the loaded handle into a PERSISTENT [`LoadedSituation`] resource that outlives
//! `Load` for the Generation consumer (E10.5) — mirroring the theme's
//! load-and-persist path exactly.
//!
//! Two tiers (mirroring the theme harness split):
//!
//! - **Tier (a)** — `MinimalPlugins` via [`GdtfTestAppBuilder`]: there is no
//!   `AssetServer`, so the situation loader registration + kick-off must no-op
//!   without panicking (the `asset_server.is_some()` guard, bevy-traps rule 1).
//!   Injecting a `GdtfTheme` drives the real theme-only transition; the app
//!   advances past `Load` and NO `LoadedSituation` is resolved (nothing to load).
//! - **Tier (b)** — `DefaultPlugins` (headless, `backends: None`) via
//!   `GdtfLoadTestAppBuilder`: a real `AssetServer` pointed at the workspace
//!   `assets/`. The good path resolves `situations/skirmish.ron` into a
//!   persistent `LoadedSituation` that survives `OnExit(Load)`.
//!
//! These are *pin-discriminating*: each assertion re-encodes one acceptance
//! criterion so a regression turns the test red.

use bevy::state::state::State;
use gdtf_app::test_support::{AppState, LoadedSituation};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::{GdtfTheme, default_theme};

/// Bounded budget for the Load orchestration plus its state-transition
/// propagation — bounded so a machine that never resolves fails instead of
/// hanging.
const LOAD_BUDGET: u32 = 32;

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

/// AC6 — tier (a): under `MinimalPlugins` there is no `AssetServer`, so entering
/// `Load` must not panic — the situation-loader registration and the kick-off
/// both guard on a missing server and no-op. The machine still advances past
/// `Load` (once a theme is injected to drive the theme-only transition), proving
/// the guard holds and nothing requires the `AssetServer`.
///
/// Pin: if the `init_ron_asset::<Situation>()` registration or the
/// `asset_server.load::<RonAsset<Situation>>(..)` kick-off ever ran without the
/// `asset_server.is_some()` / `Option<Res<AssetServer>>` guard, entering `Load`
/// (or this update) would panic instead of resting / advancing (bevy-traps rule 1).
#[test]
fn situation_loader_no_ops_cleanly_without_asset_server() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Enter Load: the situation loader registration + kick-off must no-op (no
    // AssetServer), not panic.
    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the kick-off must no-op and the machine rests in Load, not panic",
    );

    // Stand in for the theme resolve completing (no AssetServer under
    // MinimalPlugins), driving the real theme-only transition.
    app.world_mut().insert_resource(default_theme());

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_BUDGET,
    );
    assert!(
        reached_intro,
        "with a GdtfTheme present, Load must advance to Intro within {LOAD_BUDGET} updates; \
         last observed AppState was {:?}",
        app_state(&app),
    );

    // No AssetServer means nothing loads, so no LoadedSituation is ever resolved —
    // proving the headless guard short-circuits the whole situation load chain.
    assert!(
        app.world().get_resource::<LoadedSituation>().is_none(),
        "with no AssetServer the situation load chain must no-op — no LoadedSituation resolved",
    );
}

/// AC7 / AC9 — tier (b): with a real `AssetServer` rooted at the workspace
/// `assets/`, entering `Load` loads and resolves `situations/skirmish.ron` into a
/// PERSISTENT [`LoadedSituation`] that survives `OnExit(Load)` — proving the
/// loaded handle was resolved into a persistent `Situation` source available to
/// the Generation consumer (E10.5), and that the gap between 'handle held' and
/// 'resource available' is closed.
///
/// Value-agnostic per the de-brittle principle: it asserts the resource is
/// PRESENT and round-trips to a `Situation` with NON-EMPTY gangers, never a
/// pinned hp/tu/armor magnitude.
///
/// Pin: this fails if the kick-off path/loader regresses (the situation never
/// resolves, so the budget runs out), if `LoadedSituation` is never inserted on
/// the resolve success path (AC7), or if `cleanup` ever removed it on
/// `OnExit(Load)` instead of persisting it (AC7's persistence clause).
#[test]
fn real_asset_resolves_persistent_loaded_situation() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // The real situation load resolves a LoadedSituation within the budget.
    let resolved = advance_until(
        &mut app,
        |app| app.world().get_resource::<LoadedSituation>().is_some(),
        LOAD_BUDGET,
    );
    assert!(
        resolved,
        "the real situation load should resolve a LoadedSituation within {LOAD_BUDGET} updates; \
         last observed AppState was {:?}",
        app_state(&app),
    );

    // The resolved situation round-trips to a Situation with non-empty gangers
    // (presence + non-empty, never a magnitude).
    if let Some(loaded) = app.world().get_resource::<LoadedSituation>() {
        assert!(
            !loaded.gangers.is_empty(),
            "the resolved LoadedSituation must carry the authored (non-empty) gangers",
        );
    }

    // Drive past Load (gated solely on GdtfTheme — the situation does not gate it),
    // then assert the LoadedSituation SURVIVES OnExit(Load): it is the persistent
    // exception the Generation consumer reads.
    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        LOAD_BUDGET,
    );
    assert!(
        left_load,
        "Load must advance once a theme is present; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<LoadedSituation>().is_some(),
        "LoadedSituation must persist past OnExit(Load) — the Generation-consumer exception",
    );
}

/// AC7 (companion) — the situation does NOT add a transition gate: the Load→Intro
/// transition stays gated SOLELY on `GdtfTheme`. With a real `AssetServer`, the
/// machine reaches `Intro` (a theme is present) AND a `GdtfTheme` exists,
/// confirming the theme-only transition still fires alongside the situation
/// resolution on the same poll path.
///
/// Pin: this fails if the situation resolution were ever made a transition gate
/// (e.g. blocking `transition_to_intro` until `LoadedSituation` exists), which
/// would risk stranding the machine in `Load` on a slow/failed situation.
#[test]
fn real_asset_transition_stays_theme_only() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_BUDGET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro on theme-present; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the theme-only transition fired — a GdtfTheme is present",
    );
}
