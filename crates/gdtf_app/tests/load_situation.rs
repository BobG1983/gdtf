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
//!   GTW-261 made the situation a gate-blocking resource, so a headless walk seeds
//!   `LoadedSituation` itself (beside theme/tuning/weapons) to clear the gate; the
//!   app then advances past `Load` without ever resolving one from disk.
//! - **Tier (b)** — `DefaultPlugins` (headless, `backends: None`) via
//!   `GdtfLoadTestAppBuilder`: a real `AssetServer` pointed at the workspace
//!   `assets/`. The good path resolves `situations/skirmish.ron` into a
//!   persistent `LoadedSituation` that survives `OnExit(Load)`.
//!
//! These are *pin-discriminating*: each assertion re-encodes one acceptance
//! criterion so a regression turns the test red.

use std::path::PathBuf;

use bevy::state::state::State;
use gdtf_app::test_support::{AppState, LoadedSituation};
use gdtf_battle_sim::{situation::Situation, tuning::CombatTuning, weapon::WeaponRegistry};
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_ui::theme::{GdtfTheme, default_theme};

/// Bounded budget for the Tier (a) `MinimalPlugins` transition / negative waits,
/// where all gate resources are injected by hand — a true, small, deterministic
/// frame count (no async load to wait on).
const TRANSITION_BUDGET: u32 = 32;

/// Generous SAFETY-NET cap for the real-asset (Tier b) `advance_until` waits gated
/// on an async asset load resolving. The situation / theme / tuning loads share the
/// `AssetServer` with the presenter's startup tile-sheet + role-table loads (the
/// full scene stack is registered in this harness), so under parallel `cargo`
/// contention the async resolve has NO fixed frame count (a hard 32-update budget
/// proved flaky). These waits key off the resolved SIGNAL; the cap is a safety net
/// against a genuine never-resolve hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

/// AC6 / GTW-261 — tier (a): under `MinimalPlugins` there is no `AssetServer`, so
/// entering `Load` must not panic — the situation-loader registration and the kick-off
/// both guard on a missing server and no-op. GTW-261 made the situation a
/// gate-blocking resource, so the headless caller seeds `LoadedSituation` itself
/// (beside theme/tuning/weapons) to clear the gate; the machine then advances past
/// `Load` without the `AssetServer` ever resolving one from disk.
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

    // Stand in for the theme + tuning + weapons + situation resolves completing (no
    // AssetServer under MinimalPlugins), driving the real gated transition (GTW-206:
    // theme + tuning required; GTW-257: the WeaponRegistry; GTW-261: the
    // LoadedSituation, the empty-battle-race fix — the headless walk seeds the empty
    // default itself, symmetric with the other three).
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-269: the Load gate also requires an ArmorRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation(Situation::default()));

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        TRANSITION_BUDGET,
    );
    assert!(
        reached_intro,
        "with a GdtfTheme + CombatTuning + WeaponRegistry + LoadedSituation present, Load must \
         advance to Intro within {TRANSITION_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );

    // The LoadedSituation present at Intro is the SEEDED one — with no AssetServer the
    // situation load chain never resolved one from disk (the headless guard
    // short-circuits the whole load chain; the seed alone cleared the gate).
    assert!(
        app.world().get_resource::<LoadedSituation>().is_some(),
        "the seeded LoadedSituation must be the one that cleared the gate (no AssetServer resolve)",
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

    // Signal-poll the async situation load: wait until LoadedSituation is inserted
    // (the success-resolve path; the failure path inserts the empty default — same
    // signal), not a fixed frame count. Cap is a safety net (GTW-305).
    advance_until_resource_exists::<LoadedSituation>(&mut app, LOAD_SAFETY_NET);

    // The resolved situation round-trips to a Situation with non-empty gangers
    // (presence + non-empty, never a magnitude).
    if let Some(loaded) = app.world().get_resource::<LoadedSituation>() {
        assert!(
            !loaded.gangers.is_empty(),
            "the resolved LoadedSituation must carry the authored (non-empty) gangers",
        );
    }

    // Drive past Load (now gated on the situation too — GTW-261), then assert the
    // LoadedSituation SURVIVES OnExit(Load): it is the persistent exception the
    // Generation consumer reads.
    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        LOAD_SAFETY_NET,
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

/// AC2 / GTW-261 — the Load→Intro transition GATES on the situation: with a real
/// `AssetServer`, the machine reaches `Intro` only once a `GdtfTheme` AND a
/// `LoadedSituation` are both present, and the situation present at Intro is the REAL
/// shipped skirmish (non-empty gangers) — proving Load WAITED for the real situation
/// rather than racing to the empty default (the empty-battle-race fix). This reverses
/// the pre-GTW-261 "theme-only / situation-non-blocking" behavior on purpose.
///
/// Pin: this fails if the situation were dropped from the transition gate (then Load
/// could reach Intro with an empty/absent situation — the original bug), or if the
/// real situation never resolved (the gate would never clear within the budget).
#[test]
fn real_asset_gate_waits_for_the_real_situation() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once theme + situation resolve; last \
         AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the gated transition fired — a GdtfTheme is present",
    );
    // The situation present when Load cleared is the REAL shipped skirmish (non-empty),
    // NOT the empty default — proving Load waited for the real situation (AC2).
    let loaded = app.world().get_resource::<LoadedSituation>();
    assert!(
        loaded.is_some(),
        "a LoadedSituation must be present at Intro (the gate waited for it)",
    );
    if let Some(loaded) = loaded {
        assert!(
            !loaded.gangers.is_empty(),
            "the situation that cleared the gate must be the real (non-empty) skirmish, not the \
             empty default — Load waited for the real situation",
        );
    }
}

/// AC1 / GTW-261 — the gate REQUIRES the situation (the regression test). With a
/// `GdtfTheme` + `CombatTuning` + `WeaponRegistry` present but `LoadedSituation`
/// deliberately ABSENT (and no `AssetServer` to resolve one), the machine must STAY
/// in `Load`; once a `LoadedSituation` is inserted it advances past `Load`.
///
/// Pin-discriminating: this fails on the pre-GTW-261 gate (which omitted the
/// situation), where the machine would advance to Intro with no situation present —
/// the exact path that produced the empty-battle bug. Mirrors the GTW-257
/// `load_does_not_leave_without_a_weapon_registry` shape for the situation.
#[test]
fn load_does_not_leave_without_a_situation() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Theme + tuning + weapons + armor present, but the LoadedSituation deliberately
    // withheld (so the situation is the ONE missing gate resource being asserted on).
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );
    assert!(
        !left_load,
        "Load must NOT leave while the LoadedSituation is absent; it left to {:?}",
        app_state(&app),
    );
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no LoadedSituation present, the machine stays in Load (a battle never starts \
         situation-less — the empty-battle-race fix)",
    );

    // Insert the situation: now ALL four gate resources are present, so Load advances.
    app.world_mut()
        .insert_resource(LoadedSituation(Situation::default()));
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        TRANSITION_BUDGET,
    );
    assert!(
        reached_intro,
        "once a LoadedSituation is inserted, Load must advance to Intro within {TRANSITION_BUDGET} \
         updates; last observed AppState was {:?}",
        app_state(&app),
    );
}

/// The fixtures root whose `situations/skirmish.ron` is deliberately malformed
/// (`tests/fixtures/bad_situation_root`), while its `theme` / `combat` / `weapons` /
/// `fonts` dirs symlink the real `assets/` — so ONLY the situation branch reaches
/// [`Failed`](bevy::asset::LoadState::Failed) and the empty-default fallback runs.
fn bad_situation_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("bad_situation_root")
}

/// AC3 / GTW-261 — a FAILED situation falls back to the empty default and `Load`
/// still advances (no strand). With a real `AssetServer` rooted at a fixtures dir
/// whose `situations/skirmish.ron` is malformed (but valid theme/tuning/weapons/fonts,
/// so ONLY the situation branch fails), the situation load reaches `Failed`. The app
/// must NOT hang — the resolve `warn!`s, falls back to an empty `Situation::default()`
/// `LoadedSituation`, and `Load` still transitions to `Intro` within the bounded
/// budget (the no-strand guarantee preserved via the failure fallback).
///
/// Pin: this fails if a failed situation hangs the machine (the gate never clears
/// because no fallback is inserted), or if the failure path resolved a non-empty
/// situation. The empty default has zero gangers, distinguishing it from the good
/// path (which loads the shipped non-empty skirmish).
#[test]
fn real_asset_failed_situation_falls_back_and_does_not_strand() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(bad_situation_root())
        .starting_in(AppState::Load)
        .build();

    // Signal-poll: the failed situation must still produce a LoadedSituation (the empty
    // default) — the failure path inserts the default, so the same inserted-resource
    // signal fires, proving the app never hangs on a failed situation. Cap is a safety
    // net (GTW-305).
    advance_until_resource_exists::<LoadedSituation>(&mut app, LOAD_SAFETY_NET);

    // The fallback is the EMPTY default (zero gangers) — distinguishing the failure
    // path from the good path that loads the shipped non-empty skirmish.
    if let Some(loaded) = app.world().get_resource::<LoadedSituation>() {
        assert!(
            loaded.gangers.is_empty(),
            "the failure path must insert exactly the empty default situation (zero gangers)",
        );
    }

    // The machine still leaves Load for Intro — a failed situation does not strand it
    // (the valid theme/tuning/weapons resolve and the empty-default situation clears
    // the situation gate, so all four gate resources are satisfied).
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "even on a failed situation, Load must transition to Intro with the empty-default \
         situation; last AppState was {:?}",
        app_state(&app),
    );
}
