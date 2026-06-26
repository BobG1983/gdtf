//! GTW-269 (slice B): `AppState::Load` preloads the `assets/armor/` folder through
//! the `RonAsset<ArmorSpec>` loader (guarded for headless), builds a name-keyed
//! `ArmorRegistry` from the loaded armor files (keyed by filename stem), and gates the
//! Load->Intro transition on it — so the machine never leaves `Load` before the armor
//! folder is verified loaded. This mirrors the GTW-257 weapon load-and-build path
//! exactly. The registry is DORMANT after this slice (nothing consumes it yet — slice
//! C does); it is resolved and gated on purely so it is present when `Load` exits.
//!
//! Two tiers (mirroring the weapon harness split, `load_weapons.rs`):
//!
//! - **Tier (a)** — `MinimalPlugins` via [`GdtfTestAppBuilder`]: there is no
//!   `AssetServer`, so the armor-loader registration + folder kick-off must no-op
//!   without panicking (the `asset_server.is_some()` guard, bevy-traps rule 1).
//!   Injecting all five gate resources (theme + tuning + weapons + situation + armor)
//!   drives the real gated transition; the app advances past `Load`, and no registry is
//!   RESOLVED from disk (nothing to load).
//! - **Tier (b)** — `DefaultPlugins` (headless, `backends: None`) via
//!   [`GdtfLoadTestAppBuilder`]: a real `AssetServer` pointed at the workspace
//!   `assets/`. The good path loads `assets/armor/*.armor.ron` into an `ArmorRegistry`
//!   keyed by file stem (`flak_vest`, `carapace_plate`).
//!
//! These are *pin-discriminating*: each assertion re-encodes one acceptance criterion
//! so a regression turns the test red. They are VALUE-AGNOSTIC (only the registry's
//! PRESENCE + that it is keyed by the authored filename stems), so a tuning edit never
//! reddens them — the authored armor magnitudes are tuning DATA, NOT pinned by tests
//! (the brittle-test rule; see [`ArmorSpec`](gdtf_battle_sim::armor::ArmorSpec)). The
//! field-to-slot conversion MECHANISM is covered by the fixture-based sim round-trip
//! (`armor::test::armor_spec_round_trips_and_registry_resolves_by_name`); this harness
//! proves the REAL `assets/armor/` folder loads through the Load code path and that its
//! authored KEYS resolve.

use gdtf_app::test_support::{AppState, LoadedSituation, seed_load_fallbacks};
use gdtf_battle_sim::{
    armor::{ArmorName, ArmorRegistry},
    injuries::InjuryRegistry,
    situation::Situation,
    terrain::piece::TerrainRegistry,
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
/// on an async asset load resolving. The armor folder load shares the `AssetServer`
/// with the theme / situation / tuning / weapons + the presenter's startup tile-sheet
/// loads (the full scene stack is registered in this harness), so under parallel
/// `cargo` contention the async resolve has NO fixed frame count. These waits key off
/// the resolved SIGNAL; the cap is a safety net against a genuine never-resolve hang,
/// not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world()
        .resource::<bevy::state::state::State<AppState>>()
        .get()
        .clone()
}

/// AC (tier a) — under `MinimalPlugins` there is no `AssetServer`, so entering `Load`
/// must not panic: the armor-loader registration (`init_ron_asset_with_extensions::<ArmorSpec>`)
/// and the `load_folder("armor")` kick-off both guard on a missing server and no-op.
/// The machine still advances past `Load` once all five gate resources are injected
/// (standing in for all resolves completing), proving the guard holds.
///
/// Pin: if the armor-loader registration or the `load_folder` kick-off ever ran
/// without the `asset_server.is_some()` / `Option<Res<AssetServer>>` guard, entering
/// `Load` would panic instead of resting / advancing (bevy-traps rule 1).
#[test]
fn armor_loader_no_ops_cleanly_without_asset_server() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Enter Load: the armor loader registration + folder kick-off must no-op (no
    // AssetServer), not panic.
    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the kick-off must no-op and the machine rests in Load, not panic",
    );

    // Stand in for the theme + tuning + weapons + situation + armor resolves completing
    // (no AssetServer under MinimalPlugins), driving the real gated transition (GTW-269:
    // the ArmorRegistry is required before Load transitions, alongside the GTW-257
    // WeaponRegistry and the GTW-261 LoadedSituation).
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-384: the GangerStatTuning is a gate-blocking resource too (the sim derives
    // ganger stats from it), so seed it alongside the others to reach Intro.
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut().insert_resource(ArmorRegistry::default());
    // GTW-394: the Load gate also requires a TerrainRegistry; empty clears it.
    app.world_mut().insert_resource(TerrainRegistry::default());
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
        "with a GdtfTheme + CombatTuning + GangerStatTuning + WeaponRegistry + LoadedSituation \
         + ArmorRegistry + TerrainRegistry present, Load must advance to Intro within \
         {TRANSITION_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );
}

/// AC (companion, tier a) — the Load->Intro transition GATES on the `ArmorRegistry`:
/// with a theme + tuning + weapons + situation present but NO armor registry (and no
/// `AssetServer` to resolve one), the machine must stay in `Load` for the whole budget.
/// Proves the armor registry is a genuine gate-blocking resource (the machine never
/// leaves `Load` before the armor folder is verified loaded).
///
/// Pin: this fails if the transition ever fired without the armor registry present
/// (regressing the GTW-269 gate clause), which would let `Load` exit before the armor
/// folder resolved (so `spec("flak_vest")` could return `None` at battle setup).
#[test]
fn load_does_not_leave_without_an_armor_registry() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // The other gate resources present, but the ArmorRegistry deliberately withheld.
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-394: also seed TerrainRegistry so the armor gate is the only missing one.
    app.world_mut().insert_resource(TerrainRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );

    assert!(
        !left_load,
        "Load must NOT leave while the ArmorRegistry is absent; it left to {:?}",
        app_state(&app),
    );
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no ArmorRegistry present, the machine stays in Load (the armor folder must be \
         verified loaded before Load exits)",
    );
}

/// AC (tier b) / GTW-270 AC — with a real `AssetServer` rooted at the workspace
/// `assets/`, entering `Load` loads `assets/armor/*.armor.ron` and builds an
/// `ArmorRegistry` keyed by each armor file's filename stem (with the `.armor` infix
/// stripped). Proves the folder loaded into the registry keyed by filename (the
/// canonical `flak_vest` / `carapace_plate` keys resolve), and that the Load gate
/// waited for it (the machine reaches Intro with a registry present).
///
/// GTW-270: this asserts that the REAL armor folder loads + each authored KEY resolves
/// to `Some(spec)` through the Load code path (mirroring the GTW-257/269 precedent). It
/// does NOT pin any authored magnitude — those are tuning DATA (the brittle-test rule);
/// the field-to-slot conversion mechanism is covered by the fixture-based sim round-trip
/// (`armor::test::armor_spec_round_trips_and_registry_resolves_by_name`).
#[test]
fn real_asset_resolves_armor_registry_keyed_by_filename() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async armor folder load: wait until the ArmorRegistry is inserted,
    // not a fixed frame count. Cap is a safety net (GTW-305).
    advance_until_resource_exists::<ArmorRegistry>(&mut app, LOAD_SAFETY_NET);

    // The registry is keyed by the authored filename stems (with the `.armor` infix
    // stripped): `flak_vest.armor.ron` keys `flak_vest`; `carapace_plate.armor.ron` keys
    // `carapace_plate`. Each authored key resolves to its loaded spec.
    if let Some(registry) = app.world().get_resource::<ArmorRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved ArmorRegistry must carry the authored (non-empty) armor",
        );
        assert!(
            registry
                .spec(&ArmorName::new("flak_vest".to_owned()))
                .is_some(),
            "the registry must hold the `flak_vest` armor (keyed by flak_vest.armor.ron's stem)",
        );
        assert!(
            registry
                .spec(&ArmorName::new("carapace_plate".to_owned()))
                .is_some(),
            "the registry must hold the `carapace_plate` armor (keyed by carapace_plate.armor.ron's stem)",
        );
    }

    // The Load gate WAITED for the registry: the machine reaches Intro, and an
    // ArmorRegistry is present when it does (GTW-269 gate clause).
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once theme + tuning + weapons + armor \
         resolve; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<ArmorRegistry>().is_some(),
        "an ArmorRegistry must be present when Load reaches Intro (the gate waited for it)",
    );
    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the theme resolved alongside the armor (the gated transition fired)",
    );
}

/// GTW-269 (mirroring GTW-297 `AC3b`) — the REAL auto-battle path: the
/// `AutoBattlePlugin`'s `seed_load_fallbacks` runs on `Startup` AND a real
/// `AssetServer` is present (the GUI auto-battle launch). The empty
/// `ArmorRegistry::default()` seed must NOT shadow the real folder resolve: with an
/// `AssetServer` present the seed must NOT insert an empty registry, so
/// `poll_and_resolve` (which only runs `resolve_armor` while the registry is ABSENT)
/// populates it from `assets/armor/*.armor.ron`.
///
/// This reproduces the bug's exact preconditions on the real code path: a
/// `GdtfLoadTestAppBuilder` app (live `AssetServer` rooted at the workspace `assets/`)
/// with the genuine `seed_load_fallbacks` system registered on `Startup`, exactly as
/// `AutoBattlePlugin::build` wires it. The assertions encode the fix.
///
/// PIN: if the seed reverts to inserting `ArmorRegistry::default()` UNCONDITIONALLY,
/// the registry is present from `Startup`, `poll_and_resolve` SKIPS `resolve_armor`
/// (its `if !armor_present` guard), the registry stays empty, `spec("flak_vest")` returns
/// `None`, and this test goes red.
#[test]
fn seeded_startup_does_not_shadow_real_armor_resolution() {
    use bevy::app::Startup;

    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    // Wire the REAL auto-battle Startup seed (the path AutoBattlePlugin registers),
    // minus the unrelated `drive_past_menu` that needs the RunningState machinery.
    app.add_systems(Startup, seed_load_fallbacks);

    // Signal-poll the ArmorRegistry insert (not a fixed frame count): the real resolve
    // only inserts the registry ONCE it is fully built from the folder (it stays ABSENT
    // while members are still resolving — see `resolve_armor`), so on the GOOD path
    // existence implies the authored armor is present. The cap is a safety net (GTW-305).
    // The seed-shadow regression instead inserts an EMPTY registry at Startup, which the
    // assertions below catch immediately.
    advance_until_resource_exists::<ArmorRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<ArmorRegistry>() {
        // Pin: with the AssetServer present the empty seed must NOT win — the real folder
        // resolve must populate a registry that holds the authored armor.
        assert!(
            !registry.is_empty(),
            "the real folder resolve must populate the registry, not leave the empty seed",
        );
        assert!(
            registry
                .spec(&ArmorName::new("flak_vest".to_owned()))
                .is_some(),
            "the registry must hold `flak_vest` — the empty seed must NOT have shadowed the resolve",
        );
        assert!(
            registry
                .spec(&ArmorName::new("carapace_plate".to_owned()))
                .is_some(),
            "the resolved registry must also hold `carapace_plate` (the empty seed held neither)",
        );
    }

    // The gate waited for the REAL registry: the machine reaches Intro with it present.
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with the Startup seed present, Load must still reach Intro once the real armor resolves; \
         last AppState was {:?}",
        app_state(&app),
    );
}
