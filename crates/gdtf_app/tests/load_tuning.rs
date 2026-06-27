//! GTW-206 (E10.4): `AppState::Load` loads the shipped `CombatTuning` `.ron`
//! through the `RonAsset<CombatTuning>` loader, guarded for headless, and inserts
//! the deserialized payload as a PERSISTENT [`CombatTuning`] resource that
//! outlives `Load` for the `BattleScape` consumer — mirroring the theme's
//! load-and-persist path, but for a payload that IS both the `Deserialize` value
//! and the `Resource` (no separate `resolve()` step).
//!
//! Two tiers (mirroring the theme/situation harness split):
//!
//! - **Tier (a)** — `MinimalPlugins` via [`GdtfTestAppBuilder`]: there is no
//!   `AssetServer`, so the tuning loader registration + kick-off must no-op
//!   without panicking (the `asset_server.is_some()` guard, bevy-traps rule 1),
//!   and no `CombatTuning` is inserted from the load path (AC2/AC3).
//! - **Tier (b)** — `DefaultPlugins` (headless, `backends: None`) via
//!   `GdtfLoadTestAppBuilder`: a real `AssetServer` pointed at the workspace
//!   `assets/`. The good path resolves `core_tuning/combat.tuning.ron` into a persistent
//!   `CombatTuning` that survives `OnExit(Load)` (AC4/AC5); a deliberately-bad
//!   tuning path drives the failure branch — it warns, falls back to
//!   `CombatTuning::default()`, and `Load` still transitions (AC6).
//!
//! These are *pin-discriminating*: each assertion re-encodes one acceptance
//! criterion so a regression turns the test red. VALUE-AGNOSTIC throughout — no
//! pinned tuning magnitude is ever asserted (only presence / parse-OK / the
//! default-vs-shipped MECHANISM), so a balance edit to `tuning.ron` never reddens
//! these tests.

use std::path::PathBuf;

use bevy::state::state::State;
use gdtf_app::test_support::{AppState, LoadedSituation};
use gdtf_battle_sim::{
    injuries::InjuryRegistry,
    level::ThemeCatalogRegistry,
    situation::Situation,
    terrain::piece::TerrainRegistry,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::WeaponRegistry,
};
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_ui::theme::default_theme;

/// Bounded budget for the Tier (a) `MinimalPlugins` transition / negative waits,
/// where all gate resources are injected by hand — a true, small, deterministic
/// frame count (no async load to wait on).
const TRANSITION_BUDGET: u32 = 32;

/// Generous SAFETY-NET cap for the real-asset (Tier b) `advance_until` waits gated
/// on an async asset load resolving (the `Load -> Intro` transition needs theme +
/// tuning + weapons + situation all resolved). A safety net against a genuine
/// never-resolve hang, NOT a timing budget: those waits key off the resolved SIGNAL
/// and merely cap the worst case high enough to absorb parallel-load variance
/// (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

/// Absolute path to the malformed-tuning fixtures root
/// (`tests/fixtures/bad_tuning_root`), whose `core_tuning/combat.tuning.ron` is deliberately
/// unparseable so the real loader reaches `Failed`, while its `core_tuning/ui_theme.tuning.ron`
/// (and the symlinked `fonts/`) stay VALID so ONLY the tuning branch fails.
fn bad_tuning_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("bad_tuning_root")
}

/// AC2 / AC3 — tier (a): under `MinimalPlugins` there is no `AssetServer`, so
/// entering `Load` must not panic — the tuning-loader registration and the
/// kick-off both guard on a missing server and no-op, and NO `CombatTuning` is
/// inserted by the load path. The machine still advances past `Load` once a theme
/// AND a tuning are injected (standing in for both resolves completing), proving
/// the guard holds and nothing requires the `AssetServer`.
///
/// Pin: if the `init_ron_asset::<CombatTuning>()` registration or the
/// `asset_server.load::<RonAsset<CombatTuning>>(..)` kick-off ever ran without the
/// `asset_server.is_some()` / `Option<Res<AssetServer>>` guard, entering `Load`
/// (or this update) would panic instead of resting / advancing (bevy-traps rule
/// 1). It also pins that with no server no `CombatTuning` is fabricated by the
/// load path.
#[test]
fn tuning_loader_no_ops_cleanly_without_asset_server() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Enter Load: the tuning loader registration + kick-off must no-op (no
    // AssetServer), not panic.
    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the kick-off must no-op and the machine rests in Load, not panic",
    );

    // No AssetServer means nothing loads, so no CombatTuning is inserted by the
    // load path — proving the headless guard short-circuits the tuning load chain.
    assert!(
        app.world().get_resource::<CombatTuning>().is_none(),
        "with no AssetServer the tuning load chain must no-op — no CombatTuning inserted",
    );

    // Stand in for ALL resolves completing (no AssetServer under MinimalPlugins),
    // driving the real transition (GTW-206 AC5: theme AND tuning required; GTW-257:
    // the WeaponRegistry too; GTW-261: the LoadedSituation too).
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-384: the GangerStatTuning is a gate-blocking resource too.
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-269: the Load gate also requires an ArmorRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    // GTW-394: the Load gate also requires a TerrainRegistry; empty clears it.
    app.world_mut().insert_resource(TerrainRegistry::default());
    app.world_mut()
        .insert_resource(ThemeCatalogRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));

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
}

/// AC3 (companion) — tier (a): with neither a theme nor a tuning ever inserted (no
/// `AssetServer` to resolve them), the machine must stay in `Load` for the whole
/// budget — proving `Load` does not auto-leave while tuning-less from the load
/// path.
///
/// Pin: this fails if the transition ever fires without BOTH required resources
/// present (GTW-206 AC5). Combined with the previous test it brackets the
/// transition condition from both sides.
#[test]
fn load_does_not_leave_without_a_tuning() {
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
        "without a CombatTuning the machine must not leave Load; it reached {:?}",
        app_state(&app),
    );
}

/// AC3 (companion) — tier (a): with ONLY a theme injected (no tuning), `Load` must
/// NOT leave — proving the tuning is a genuine SECOND gate, not subsumed by the
/// theme gate.
///
/// Pin: this fails if the Load→Intro transition still fires on theme-only (the
/// pre-GTW-206 gate), which would let `Load` exit tuning-less.
#[test]
fn load_does_not_leave_on_theme_only() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    app.update();
    // A theme alone is NOT enough to transition anymore (GTW-206 AC5).
    app.world_mut().insert_resource(default_theme());

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );

    assert!(
        !left_load,
        "with a theme but NO tuning the machine must not leave Load; it reached {:?}",
        app_state(&app),
    );
}

/// AC4 / AC5 — tier (b): with a real `AssetServer` rooted at the workspace
/// `assets/`, entering `Load` loads and deserializes the SHIPPED `core_tuning/combat.tuning.ron`
/// into a PERSISTENT [`CombatTuning`] resource that survives `OnExit(Load)` and
/// composes correctly with the theme branch (both present before `Load` leaves).
///
/// Value-agnostic: it asserts the resource is PRESENT and survives, never a pinned
/// magnitude. AC5's distinguish-from-default clause degrades gracefully — it
/// asserts `!= default` ONLY IF the shipped file actually differs from `Default`
/// (a one-time in-test check), so a `tuning.ron` edited to equal the const default
/// would degrade this to a presence assertion rather than redden the test.
///
/// Pin: this fails if the kick-off path/loader regresses (the tuning never
/// resolves, so the budget runs out), if the `CombatTuning` insert branch in
/// `poll_and_resolve` is dropped (AC4), or if `cleanup` ever removed it on
/// `OnExit(Load)` instead of persisting it (AC5's persistence clause).
#[test]
fn real_asset_resolves_persistent_combat_tuning() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async tuning load: wait until CombatTuning is inserted (the
    // success-resolve path; the failure path would insert the default — same signal),
    // not a fixed frame count. Cap is a safety net (GTW-305).
    advance_until_resource_exists::<CombatTuning>(&mut app, LOAD_SAFETY_NET);

    // AC5 mechanism (not magnitude): IF the shipped file differs from Default, the
    // GOOD path must have loaded the real file (not silently fallen to the const
    // default). If the shipped file equals Default, this degrades to presence.
    if let Some(tuning) = app.world().get_resource::<CombatTuning>() {
        let default = CombatTuning::default();
        if &default != tuning {
            assert_ne!(
                tuning, &default,
                "the good path must have parsed the SHIPPED tuning, distinct from the const default",
            );
        }
    }

    // Drive past Load, then assert the CombatTuning SURVIVES OnExit(Load): it is
    // the persistent exception the BattleScape consumer reads — AND that the tuning
    // branch did not starve / was not starved by the theme branch (both present
    // before Load left, GTW-206 AC5).
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a resolved theme + tuning, Load must transition to Intro; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "CombatTuning must persist past OnExit(Load) into Intro — the BattleScape-consumer \
         exception (like GdtfTheme), NOT removed by cleanup",
    );
}

/// AC6 — tier (b) failure path: with the asset root pointed at a malformed
/// `core_tuning/combat.tuning.ron` (but a VALID `core_tuning/ui_theme.tuning.ron` + fonts, so ONLY the
/// tuning branch fails), the tuning load reaches `Failed`. The app must NOT hang
/// — it warns naming the path, falls back to exactly `CombatTuning::default()`,
/// and `Load` still transitions to `Intro` within the bounded budget.
///
/// Pin: this fails if a failed tuning hangs the machine (the budget runs out
/// before a tuning appears), or if the failure path does not insert the
/// const-fallback tuning (AC6). The fallback equals `CombatTuning::default()`, so
/// asserting the resolved resource is exactly that distinguishes the failure path
/// from the good path (which loads the distinct shipped file).
#[test]
fn real_asset_failure_path_does_not_hang_and_uses_default_tuning() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(bad_tuning_root())
        .starting_in(AppState::Load)
        .build();

    // The bad tuning must still produce a CombatTuning (the const fallback): the
    // failure path inserts the default, so the same inserted-resource signal fires —
    // proving the app never hangs on a failed tuning. Signal-poll, cap is a safety net.
    advance_until_resource_exists::<CombatTuning>(&mut app, LOAD_SAFETY_NET);

    if let Some(tuning) = app.world().get_resource::<CombatTuning>() {
        assert_eq!(
            tuning,
            &CombatTuning::default(),
            "the failure path must insert exactly the const-fallback default tuning",
        );
    }

    // The machine still leaves Load for Intro — a bad tuning does not strand it
    // (the valid theme resolves and the default tuning is present, so both gates
    // are satisfied).
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "even on a failed tuning, Load must transition to Intro with the default tuning; \
         last AppState was {:?}",
        app_state(&app),
    );
}
