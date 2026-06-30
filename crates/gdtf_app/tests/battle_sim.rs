//! GTW-207 (E10.5): `BattleSimPlugin` drives the render-free authoritative sim
//! into the running app — on entry to `BattleScapeState::Generation` it seeds the
//! battle RNG streams (GTW-14), builds the battle from the authored `Situation` via the
//! authoritative `setup_battle`, gates Generation's state advance on REAL setup
//! success, and cleans the battle-lifetime resources only when the battle ends.
//!
//! All tests are headless `MinimalPlugins` (via [`GdtfTestAppBuilder`]) — they
//! BYPASS the `Load` scene, so each injects the persistent `Load` resources it
//! relies on (a `GdtfTheme` + `CombatTuning` to pass the Load gate, and where the
//! test exercises a specific battlefield, a `LoadedSituation` fixture). They are
//! *pin-discriminating*: each assertion re-encodes one acceptance criterion so a
//! regression turns the test red.

use std::{
    io::Write,
    sync::{Arc, LazyLock, Mutex},
};

use bevy::{
    log::tracing_subscriber::{self, fmt::MakeWriter, util::SubscriberInitExt as _},
    state::state::State,
};
use gdtf_app::test_support::{
    BattleRunningComplete, BattleScapeState, GameState, LoadedSituation, RunningState,
};
use gdtf_battle_sim::{
    armor::Wears,
    battle::BattleInProgress,
    cover::CoverLedger,
    injuries::InjuryRegistry,
    occupancy::OccupancyGrid,
    rng::{BattleSeed, ShotRng},
    situation::Situation,
    surface::SurfaceGrid,
    test_support::{
        SituationBuilder, ganger_at, key, test_armor_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::CombatTuning,
    vertical::{LinkKind, VerticalLink, VerticalLinkGraph},
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk down into the battlescape (each
/// leaf scene spends a couple of `FixedUpdate` ticks plus its transition
/// propagation), but bounded so a machine that never reaches the predicate fails
/// instead of hanging.
const BUDGET: u32 = 96;

/// A valid two-ganger fixture situation (no cover / slabs / links needed — a
/// link-free situation validates trivially), built over the central
/// [`SituationBuilder`](gdtf_battle_sim::test_support::SituationBuilder) +
/// [`ganger_at`](gdtf_battle_sim::test_support::ganger_at).
fn two_ganger_situation() -> Situation {
    SituationBuilder::new()
        .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)])
        .build()
}

/// A situation with a DANGLING vertical link (an endpoint at a `(cell, level)` no
/// authored tile occupies) — `setup_battle` returns `Err(DanglingCell)` and inserts
/// no resource (the `setup_aborts_on_invalid_vertical_link` precedent). Built entirely
/// over the central
/// [`SituationBuilder`](gdtf_battle_sim::test_support::SituationBuilder), whose
/// [`vertical_link`](gdtf_battle_sim::test_support::SituationBuilder::vertical_link)
/// setter authors the deliberately-bad link this validation-abort test reaches for.
fn dangling_link_situation() -> Situation {
    let present = key(4, 4, 0);
    let missing = key(4, 4, 1); // never authored — the link dangles off it
    SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .slab_at(present) // only `present` authored; `missing` dangles
        .vertical_link(VerticalLink::new(present, missing, LinkKind::stair()))
        .build()
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Stands in for the player at the menu (it no longer auto-advances, GTW-121):
/// advances until [`RunningState::Menu`] rests, then queues `Menu → Options`.
fn drive_past_menu(app: &mut bevy::app::App) -> bool {
    let reached = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if reached {
        app.world_mut()
            .resource_mut::<bevy::state::state::NextState<RunningState>>()
            .set(RunningState::Options);
    }
    reached
}

/// Builds the headless walk app, injecting the persistent `Load` resources the
/// machine needs to traverse `Load` (no `AssetServer` under `MinimalPlugins`), plus
/// a `LoadedSituation` for the Generation setup to consume.
///
/// GTW-261 made the situation a gate-blocking `Load` resource, so a `LoadedSituation`
/// is ALWAYS seeded (symmetric with the theme/tuning/weapons seeds): the passed
/// `situation` fixture when `Some`, else the empty `Situation::default()`. The empty
/// default exercises the zero-ganger battle-build path the way the absent
/// `request_battle_setup` fallback used to.
fn walk_app(situation: Option<Situation>) -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .default_start()
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // The Load-built WeaponRegistry (GTW-257): persistent `Load` state the real app
    // resolves from assets/content/weapons/ranged/, injected here for the MinimalPlugins deep-walk
    // (no AssetServer) so the Generation setup arms each ganger from it — the canonical
    // `test_weapon_registry` (GTW-324), which holds the `test-weapon` key every fixture
    // ganger references.
    app.world_mut().insert_resource(test_weapon_registry());
    app.world_mut()
        .insert_resource(test_melee_weapon_registry());
    // The Load-built ArmorRegistry (GTW-269) so the Generation setup armors each
    // ganger: every fixture ganger references the central `test-armor` key, which the
    // canonical `test_armor_registry` (GTW-324) holds (it must be populated now that
    // setup_battle resolves armor keys; the empty-default situation has zero gangers, so
    // even then this registry is harmless).
    app.world_mut().insert_resource(test_armor_registry());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load→Intro gate also requires a GangRegistry, AND the v2 setup_battle
    // resolves each fixture ganger's (gang, member) ref against it — so seed the canonical
    // `test_gang_registry` (which holds every `ganger_at` / default-builder member), NOT an
    // empty registry (an empty one would fail closed with GangNotFound and spawn nothing).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::test_support::test_gang_registry());
    // GTW-261: the Load→Intro gate now requires a LoadedSituation; seed the fixture
    // when given, else the empty default so the walk still traverses Load.
    app.world_mut()
        .insert_resource(LoadedSituation::new(situation.unwrap_or_default()));
    app
}

/// Drives the app from the default start down to `BattleScapeState::Generation`.
/// Returns whether Generation was reached within budget.
fn drive_to_generation(app: &mut bevy::app::App) -> bool {
    if !drive_past_menu(app) {
        return false;
    }
    advance_until(
        app,
        |app| battlescape_state(app) == Some(BattleScapeState::Generation),
        BUDGET,
    )
}

/// AC1 — `BattleSimPlugin` adds `OccupancyMaintenancePlugin`: building the
/// battlescape plugin tree registers the `CoverDestroyed` message buffer (added
/// exactly once — no double-add panic), proving the reused maintenance layer is
/// wired and live.
#[test]
fn occupancy_maintenance_plugin_is_wired() {
    // Build the app (no need to drive — the plugin tree, and thus its message
    // registration, exists from construction).
    let app = walk_app(None);

    assert!(
        app.world()
            .get_resource::<bevy::ecs::message::Messages<gdtf_battle_sim::occupancy_sync::CoverDestroyed>>()
            .is_some(),
        "BattleSimPlugin must register the CoverDestroyed message buffer via \
         OccupancyMaintenancePlugin",
    );
}

/// AC2 — entering Generation seeds the five battle RNG streams (GTW-14), and the
/// seed is threaded through `ShotRng::from_root`. Two sub-properties:
///
/// (a) **Presence**: Generation inserts a `ShotRng` resource.
/// (b) **Seed sensitivity**: `ShotRng::from_root` of different seeds draws different
///     first values — the derivation `fnv1a64(seed_bytes ++ LABEL)` distinguishes seeds.
///
/// The exact seed the app chose (wall-clock or env-pinned) is not checked here —
/// that threading property is owned by the sim-level lifecycle tests
/// (`setup_threads_the_message_seed_through_rng_streams`). The app-level concern
/// is that Generation actually inserts the resource and that `from_root` is
/// seed-discriminating.
#[test]
fn entering_generation_seeds_rng_streams() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach BattleScapeState::Generation within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // (a) The stream resource is present after Generation is reached.
    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "entering Generation must insert the ShotRng stream (GTW-14 five-stream setup)",
    );

    // (b) Seed sensitivity: two different seeds produce different first draws —
    // the derivation path (FNV-1a-64 + seed_from_u64 + ChaCha12) is sensitive to
    // the seed. This property holds without knowing which seed the app chose.
    let mut seed_zero = ShotRng::from_root(BattleSeed::new(0));
    let mut seed_one = ShotRng::from_root(BattleSeed::new(1));
    assert_ne!(
        seed_zero.next_u64(),
        seed_one.next_u64(),
        "different BattleSeeds must yield different ShotRng first draws",
    );
    // Reproducibility: the same seed produces the same first draw across two constructions.
    let mut a = ShotRng::from_root(BattleSeed::new(42));
    let mut b = ShotRng::from_root(BattleSeed::new(42));
    assert_eq!(
        a.next_u64(),
        b.next_u64(),
        "same BattleSeed must reproduce the same ShotRng draw",
    );
}

/// AC3 — `setup_battle` runs on the real `Commands` path: its four resources land
/// in the world, and the authored ganger count equals the spawned `Wears`-carrying
/// ganger count (the armor relationship) — proving the real setup ran, not a stub.
#[test]
fn setup_battle_lands_resources_and_spawns_gangers() {
    let situation = two_ganger_situation();
    let authored_gangers = situation.gangers.len();
    let mut app = walk_app(Some(situation));
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach Generation within {BUDGET} updates",
    );

    // All four setup_battle resources are present.
    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "setup_battle must insert a CoverLedger",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "setup_battle must insert a SurfaceGrid",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "setup_battle must insert an OccupancyGrid",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "setup_battle must insert a VerticalLinkGraph",
    );

    // The spawned ganger count equals the authored count (each ganger carries the
    // `Wears` armor relationship since GTW-323): exactly the fixture's gangers were
    // spawned on the real path. The ganger + its `Wears` armor-piece entities spawn as
    // `bsn!` scenes (`queue_spawn_related_scenes::<Wears>`), deferred to the `SpawnScene`
    // schedule; the related-piece spawn lands a frame after the ganger scene, so settle
    // one update so the `WornBy` back-reference hook has populated each ganger's `Wears`.
    app.update();
    let world = app.world_mut();
    let mut query = world.query::<&Wears>();
    assert_eq!(
        query.iter(world).count(),
        authored_gangers,
        "the spawned ganger count (each wearing armor via Wears) must equal the authored count",
    );
}

/// AC4 — Generation completion is GATED on real setup success: the state advances
/// to `AnimateIn`, and on the first update where the setup witness exists the gate
/// can fire — completion never precedes setup (the successful setup inserts the
/// `BattleInProgress` witness, GTW-212's explicit battle-active tag). Ordering
/// relation, not a frame count.
#[test]
fn generation_completion_is_gated_on_setup() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach Generation within {BUDGET} updates",
    );

    // Once Generation is reached the setup has succeeded, so the battle-active witness
    // exists; the gate (and thus move_on) then advances strictly after setup. Advancing
    // must reach AnimateIn, and the BattleInProgress witness must already be present (it
    // was inserted on the same successful-setup Ok path that signals BattleReady — the
    // sim-side band keys off it, GTW-212).
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "a successful setup must have inserted the BattleInProgress witness before completion gates",
    );

    let reached_animate_in = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::AnimateIn),
        BUDGET,
    );
    assert!(
        reached_animate_in,
        "with setup succeeded, Generation must advance to AnimateIn within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
}

/// AC5 — a FAILED setup does NOT gate Generation complete (no panic, no silent
/// advance): a dangling vertical link makes `setup_battle` return `Err`; the plugin
/// logs it (no panic) and inserts NO resource, so the gate never fires, the state
/// stays in Generation, and no setup resource was inserted.
#[test]
fn failed_setup_does_not_advance_generation() {
    let mut app = walk_app(Some(dangling_link_situation()));
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach Generation within {BUDGET} updates",
    );

    // The marker must never appear (setup_battle aborted before inserting any
    // resource, so the gate's witness is absent).
    let advanced = advance_until(
        &mut app,
        |app| battlescape_state(app) != Some(BattleScapeState::Generation),
        BUDGET,
    );
    assert!(
        !advanced,
        "a failed setup must NOT advance past Generation; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::Generation),
        "the machine must remain in Generation when setup failed",
    );

    // No setup resource was inserted (validation aborts before any insert).
    assert!(
        app.world().get_resource::<CoverLedger>().is_none(),
        "a failed setup must insert no CoverLedger (it aborts before any resource insert)",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_none(),
        "a failed setup must insert no OccupancyGrid",
    );
}

/// AC6 — the battle-lifetime resources SURVIVE past Generation and are cleaned
/// ONLY on leaving the battle (`GameState::BattleScape`), while `CombatTuning`
/// (E10.4's persistent Load resource) is never touched.
#[test]
fn battle_resources_survive_battle_and_clean_on_exit() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach Generation within {BUDGET} updates",
    );

    // (a) Advance PAST Generation (into AnimateIn / BattleRunning) and assert the
    //     battle-lifetime resources all still exist — proving they survive past the
    //     Generation sub-state for the E10.6 acts — AND CombatTuning persists.
    let past_generation = advance_until(
        &mut app,
        |app| {
            matches!(
                battlescape_state(app),
                Some(BattleScapeState::AnimateIn | BattleScapeState::BattleRunning)
            )
        },
        BUDGET,
    );
    assert!(
        past_generation,
        "the walk should advance past Generation into AnimateIn/BattleRunning within {BUDGET} \
         updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "RNG streams must survive past Generation (battle-lifetime)",
    );
    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "CoverLedger must survive past Generation",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "SurfaceGrid must survive past Generation",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "OccupancyGrid must survive past Generation",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "VerticalLinkGraph must survive past Generation",
    );
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "CombatTuning (E10.4's persistent Load resource) must still be present",
    );

    // (b) Advance until the machine has LEFT GameState::BattleScape, then assert the
    //     battle-lifetime resources are gone (cleaned at the battle boundary) while
    //     CombatTuning STILL persists (untouched by this plugin).
    //
    //     The battlescape now PERSISTS in BattleRunning (GTW-236, the placeholder budget
    //     auto-exit is gone), so insert the explicit `BattleRunningComplete` end-signal
    //     marker (standing in for the not-yet-wired victory/flee). Once the machine reaches
    //     BattleRunning the marker trips `move_on` and the chain advances out of the scape.
    app.world_mut().insert_resource(BattleRunningComplete);
    let left_battlescape = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<GameState>>()
                .is_none_or(|state| *state.get() != GameState::BattleScape)
        },
        BUDGET,
    );
    assert!(
        left_battlescape,
        "the walk should leave GameState::BattleScape within {BUDGET} updates",
    );
    assert!(
        app.world().get_resource::<ShotRng>().is_none(),
        "RNG streams must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<CoverLedger>().is_none(),
        "CoverLedger must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_none(),
        "SurfaceGrid must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_none(),
        "OccupancyGrid must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_none(),
        "VerticalLinkGraph must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "CombatTuning must NOT be removed by this plugin (it is the persistent Load resource)",
    );
}

/// AC8 — the empty-`Situation` build keeps the deep walk green: with an EMPTY
/// `LoadedSituation` (the `Situation::default()` the GTW-261 gate now requires the
/// walk to seed), `setup_battle(&Situation::default())` returns Ok with zero gangers,
/// the four sim resources are inserted (empty grids), the gate fires, and Generation
/// ADVANCES (it does not hang). The landed `state_walk` deep walk covers the
/// reach-Teardown half; this asserts the empty-situation setup half (the same
/// zero-ganger build the `request_battle_setup` belt-and-suspenders fallback yields).
#[test]
fn empty_situation_builds_and_advances() {
    // The empty default LoadedSituation — the MinimalPlugins default-start path now
    // seeds it (GTW-261), exercising the zero-ganger battle build.
    let mut app = walk_app(None);
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach Generation within {BUDGET} updates with the empty default situation",
    );

    // The Default (empty) situation still builds: the four sim resources are inserted.
    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "the empty Default situation must still insert a CoverLedger",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "the empty Default situation must still insert a SurfaceGrid",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "the empty Default situation must still insert an OccupancyGrid",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "the empty Default situation must still insert a VerticalLinkGraph",
    );

    // No gangers spawned (empty battlefield).
    let world = app.world_mut();
    let mut query = world.query::<&Wears>();
    assert_eq!(
        query.iter(world).count(),
        0,
        "the empty Default situation spawns zero gangers",
    );

    // And Generation still ADVANCES (the gate fired on the empty setup's resources).
    let reached_animate_in = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::AnimateIn),
        BUDGET,
    );
    assert!(
        reached_animate_in,
        "the absent-Situation Default path must still advance Generation to AnimateIn; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
}

// ---------------------------------------------------------------------------
// GTW-14 / C7 (M3): end-to-end coverage of the composition-root seed resolution.
//
// Every other app test pre-injects a `BattleSeed` via `with_seed`, so
// `request_battle_setup` takes the override branch and BYPASSES `resolve_root_seed`
// entirely. These tests drive the REAL resolution path: build WITHOUT a seed override
// so `request_battle_setup` calls `resolve_root_seed`, which reads `GDTF_BATTLE_SEED`
// (or falls back to wall-clock) and the unconditional setup `info!` (m2) fires.
// ---------------------------------------------------------------------------

/// Process-wide log capture, installed ONCE for this test binary. Bevy's
/// `MinimalPlugins` harness installs no `LogPlugin`, so this is the only global
/// tracing subscriber; it records every event into a shared buffer so the
/// seed-resolution test can prove the unconditional setup `info!` (m2) fires.
static LOG_CAPTURE: LazyLock<Arc<Mutex<Vec<u8>>>> = LazyLock::new(|| {
    let buf = Arc::new(Mutex::new(Vec::new()));
    // Install once for the binary; a later/competing subscriber makes this a no-op
    // that `try_init` reports as `Err` — harmless, the bound value is just dropped.
    let _installed = tracing_subscriber::fmt()
        .with_writer(CaptureWriter(Arc::clone(&buf)))
        .with_ansi(false)
        .with_max_level(tracing_subscriber::filter::LevelFilter::INFO)
        .finish()
        .try_init();
    buf
});

/// A `MakeWriter` that appends all formatted log output into a shared byte buffer.
#[derive(Clone)]
struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

impl<'a> MakeWriter<'a> for CaptureWriter {
    type Writer = CaptureSink;

    fn make_writer(&'a self) -> Self::Writer {
        CaptureSink(Arc::clone(&self.0))
    }
}

/// The per-write sink handed out by [`CaptureWriter`]; pushes bytes into the buffer.
struct CaptureSink(Arc<Mutex<Vec<u8>>>);

impl Write for CaptureSink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut guard) = self.0.lock() {
            guard.extend_from_slice(bytes);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Snapshot the captured log text so far.
fn captured_log() -> String {
    LOG_CAPTURE
        .lock()
        .map(|guard| String::from_utf8_lossy(&guard).into_owned())
        .unwrap_or_default()
}

/// Set the `GDTF_BATTLE_SEED` env var for the duration of the env-pinned case.
///
/// `std::env::set_var` is `unsafe` in edition 2024 because the process environment
/// is global and not thread-safe; the single serialized test below restores the var
/// immediately after the pinned drive (no other observer reads it in between).
#[allow(
    unsafe_code,
    reason = "env-var test must mutate the process environment"
)]
fn set_seed_env(key: &str, value: &str) {
    // SAFETY: serialized, single `#[test]` use; the var is removed right after the
    // env-pinned drive (see `clear_seed_env`), so no concurrent reader observes it.
    unsafe { std::env::set_var(key, value) };
}

/// Remove the `GDTF_BATTLE_SEED` env var (see [`set_seed_env`]).
#[allow(
    unsafe_code,
    reason = "env-var test must mutate the process environment"
)]
fn clear_seed_env(key: &str) {
    // SAFETY: serialized, single `#[test]` use; restores the process environment.
    unsafe { std::env::remove_var(key) };
}

/// GTW-14 / C7 (M3) — `resolve_root_seed` drives the streams end-to-end and the
/// resolved seed is logged. BOTH cases run in ONE serialized test because
/// `GDTF_BATTLE_SEED` is process-global; the var is restored between them.
///
/// (a) **env-pinned**: with `GDTF_BATTLE_SEED` set and NO `with_seed` override, the
///     world's resolved `ShotRng` first draw equals
///     `ShotRng::from_root(BattleSeed::new(<that value>))` — proving the env seed
///     actually drove the per-subsystem streams (not the override path).
/// (b) **unset**: a NON-ZERO (wall-clock) seed is still produced — the resolved
///     stream differs from the zero-seed stream — and the unconditional replay-handle
///     `info!` (m2) fired at battle setup (captured via [`LOG_CAPTURE`]).
#[test]
fn resolve_root_seed_drives_streams_and_logs() {
    // Mirrors the (private) production const in `…/battle_sim/seed.rs`.
    const SEED_ENV_VAR: &str = "GDTF_BATTLE_SEED";
    // A fixed, distinctive seed `resolve_root_seed` must parse and thread through.
    const PINNED: u64 = 0x0BAD_F00D_DEAD_BEEF;

    // Force the global log-capture subscriber up before any app drives, so the
    // unconditional setup `info!` is recorded for the case-(b) assertion below.
    LazyLock::force(&LOG_CAPTURE);

    // --- (a) env-pinned: GDTF_BATTLE_SEED drives the actual streams ---
    set_seed_env(SEED_ENV_VAR, &PINNED.to_string());
    let mut app = walk_app(None); // NO with_seed → request_battle_setup calls resolve_root_seed
    let reached = drive_to_generation(&mut app);
    // Restore the global env var immediately — before any assert and before case (b).
    clear_seed_env(SEED_ENV_VAR);
    assert!(
        reached,
        "the env-pinned walk should reach Generation within {BUDGET} updates; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "the env-pinned Generation must insert a ShotRng stream (resolve_root_seed path)",
    );

    // The world's resolved ShotRng (untouched — no fire act ran) must be byte-identical
    // to a fresh stream derived from the pinned env seed: GDTF_BATTLE_SEED drove it.
    let world_first = app.world_mut().resource_mut::<ShotRng>().next_u64();
    let mut expected = ShotRng::from_root(BattleSeed::new(PINNED));
    assert_eq!(
        world_first,
        expected.next_u64(),
        "the resolved ShotRng's first draw must equal \
         ShotRng::from_root(BattleSeed::new(PINNED)).next_u64() — proving GDTF_BATTLE_SEED drove \
         the streams end-to-end through resolve_root_seed (not the with_seed override)",
    );

    // --- (b) unset: a non-zero (wall-clock) seed is still produced AND logged ---
    let mut app = walk_app(None);
    assert!(
        drive_to_generation(&mut app),
        "the unset (wall-clock) walk should reach Generation within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    // A non-zero, time-derived seed was produced: its stream is NOT the zero-seed
    // stream (the wall-clock seed is microseconds-since-epoch, never 0).
    let world_first = app.world_mut().resource_mut::<ShotRng>().next_u64();
    let mut zero_stream = ShotRng::from_root(BattleSeed::new(0));
    assert_ne!(
        world_first,
        zero_stream.next_u64(),
        "the unset path must still derive a NON-ZERO wall-clock seed (its stream must differ from \
         the zero-seed stream)",
    );
    // And the unconditional replay-handle info! (m2) fired at battle setup.
    let logs = captured_log();
    assert!(
        logs.contains("resolved BattleSeed"),
        "battle setup must log the resolved BattleSeed (the replay handle) at info!; the captured \
         log did not contain the expected line. Captured:\n{logs}",
    );
}
