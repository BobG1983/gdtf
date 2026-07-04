//! GTW-575 headless integration test: the scene SCAFFOLD helpers on the REAL
//! `AppState` machine.
//!
//! Two contracts are pinned here, both driven through
//! [`GdtfTestAppBuilder`] (verification.md rule 3 — real transitions, not a
//! copy):
//!
//! 1. **The state-scoped-resource helper** (`gdtf_state_scoped`,
//!    `init_state_scoped_resource`): a probe resource scoped to
//!    [`AppState::Load`] is (a) ABSENT before `Load` is entered, (b) PRESENT
//!    with its exact seeded value while the machine rests in `Load`, and (c)
//!    REMOVED once `Load` exits to `Intro`.
//! 2. **The marker scaffold** (`states::scaffold` — the swept
//!    `insert_completion_marker` / `advance_state_to` registrations): the walk
//!    reaching `Load`, `Intro`, and then `Running` at all IS the proof that
//!    each scene's helper-inserted completion marker appeared and its
//!    marker-gated helper transition advanced the state (d) — `Init` and
//!    `Intro` have no other exit path.
//!
//! The broader walk pins (every transition target, the rests, the terminal
//! pop-out) stay in `tests/state_walk.rs`; this file is the scaffold-shaped
//! slice.

use bevy::{app::App, ecs::resource::Resource, state::state::State};
use gdtf_app::test_support::{AppState, LoadedSituation};
use gdtf_battle_sim::{
    injuries::InjuryRegistry,
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::WeaponRegistry,
};
use gdtf_state_scoped::StateScopedResourceAppExt as _;
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};

/// A budget large enough for the `Init → Load → Intro → Running` head of the
/// walk (each leaf scene spends a couple of `FixedUpdate` ticks plus its
/// state-transition propagation), but bounded so a stuck machine fails instead
/// of hanging.
const WALK_BUDGET: u32 = 64;

/// The probe resource the test scopes to [`AppState::Load`] through the shared
/// helper. A newtype with a distinguishing payload, so the in-state assertion
/// proves the SEED VALUE arrived — not merely that some resource exists.
#[derive(Resource, Debug, PartialEq, Eq)]
struct LoadScopedProbe(u8);

impl LoadScopedProbe {
    /// The seed constructor handed to `init_state_scoped_resource`.
    const fn seeded() -> Self {
        Self(0xA5)
    }
}

/// Builds the default-start headless walk app, registers the Load-scoped probe
/// through the shared helper, and seeds every `Load → Intro` gate resource
/// (the same stand-in-for-the-resolved-loads recipe as
/// `state_walk::walk_app_with_theme` — the `MinimalPlugins` walk has no real
/// asset loads to resolve them).
fn scaffold_walk_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .default_start()
        .build();

    // The unit under test: ONE call registers the OnEnter(Load) insert (seeded
    // by the probe's constructor) + the OnExit(Load) remove.
    app.init_state_scoped_resource(AppState::Load, LoadScopedProbe::seeded);

    // The Load → Intro gate seeds (GTW-143/206/257/261/269/384/415/437/487/489/505/549).
    app.world_mut()
        .insert_resource(gdtf_ui::theme::default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::AttachmentRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));
    app
}

/// Reads the current [`AppState`] (`Clone` but not `Copy`).
fn app_state(app: &App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

/// The scoped probe is absent before `Load`, present with its seeded value in
/// `Load`, and removed after `Load` exits — and the marker-scaffold scenes
/// advance the machine through each boundary.
#[test]
fn state_scoped_probe_lives_exactly_across_the_load_span() {
    let mut app = scaffold_walk_app();

    // (a) BEFORE ENTER: the first update leaves the machine resting in `Init`
    // (its completion marker lands via a deferred command, so the advance takes
    // further ticks) — the Load-scoped probe must not exist yet.
    app.update();
    assert_eq!(app_state(&app), AppState::Init, "the walk starts in Init");
    assert!(
        app.world().get_resource::<LoadScopedProbe>().is_none(),
        "the Load-scoped probe must be ABSENT before Load is entered",
    );

    // (d) THE MARKER SCAFFOLD ADVANCES: Init's only exit is its scaffold pair —
    // the helper-registered `insert_completion_marker::<InitComplete>` followed
    // by the marker-gated `advance_state_to(AppState::Load)` — so reaching
    // `Load` proves the completion marker appeared and drove the transition.
    let reached_load = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Load,
        WALK_BUDGET,
    );
    assert!(
        reached_load,
        "Init's marker scaffold must advance the walk to Load"
    );

    // (b) IN-STATE: the OnEnter(Load) insert ran with the caller-supplied seed.
    assert_eq!(
        app.world().get_resource::<LoadScopedProbe>(),
        Some(&LoadScopedProbe::seeded()),
        "the probe must be PRESENT with its exact seeded value while in Load",
    );

    // (c) AFTER EXIT: with every gate resource pre-seeded, Load transitions to
    // Intro; the OnExit(Load) remove must have dropped the probe.
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        WALK_BUDGET,
    );
    assert!(
        reached_intro,
        "the seeded gates must let Load advance to Intro"
    );
    assert!(
        app.world().get_resource::<LoadScopedProbe>().is_none(),
        "the probe must be REMOVED once Load exits",
    );

    // (d), again on a fully helper-registered scene: Intro's only exit is ITS
    // scaffold pair (`insert_completion_marker::<IntroComplete>` +
    // `advance_state_to(AppState::Running)`), so reaching Running pins the
    // marker-appears → state-advances contract end to end.
    let reached_running = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Running,
        WALK_BUDGET,
    );
    assert!(
        reached_running,
        "Intro's marker scaffold must advance the walk to Running",
    );
}
