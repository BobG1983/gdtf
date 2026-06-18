//! GTW-207 (E10.5): `BattleSimPlugin` drives the render-free authoritative sim
//! into the running app — on entry to `BattleScapeState::Generation` it seeds the
//! battle `SimRng`, builds the battle from the authored `Situation` via the
//! authoritative `setup_battle`, gates Generation's state advance on REAL setup
//! success, and cleans the battle-lifetime resources only when the battle ends.
//!
//! All tests are headless `MinimalPlugins` (via [`GdtfTestAppBuilder`]) — they
//! BYPASS the `Load` scene, so each injects the persistent `Load` resources it
//! relies on (a `GdtfTheme` + `CombatTuning` to pass the Load gate, and where the
//! test exercises a specific battlefield, a `LoadedSituation` fixture). They are
//! *pin-discriminating*: each assertion re-encodes one acceptance criterion so a
//! regression turns the test red.

use bevy::state::state::State;
use gdtf_app::test_support::{
    BattleRunningComplete, BattleScapeState, GameState, LoadedSituation, RunningState,
};
use gdtf_battle_sim::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        SourceArmor, WornArmor,
    },
    battle::BattleInProgress,
    cover::CoverLedger,
    ganger::{
        Aiming, Direction, Facing, Faction, GangerName, Hp, HpMax, LifeState, Luck, Shooting,
        Stance, StanceKind, Toughness, Tu, TuMax, Wounds, WoundsMax,
    },
    magazine::{Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    rng::{BattleSeed, SimRng},
    situation::{GangerSpawn, Situation},
    surface::SurfaceGrid,
    tuning::CombatTuning,
    vertical::{LinkKind, VerticalLink, VerticalLinkGraph},
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireMode, FireModeSpec, Kickback,
        MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Stable, WeaponDamage,
        WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
    },
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// The weapon KEY every fixture ganger references — present in [`weapon_registry`].
const TEST_WEAPON_KEY: &str = "test-weapon";

/// A registry holding the one [`TEST_WEAPON_KEY`] weapon the fixture gangers
/// reference, standing in for the app's `Load`-built registry (GTW-257) so the
/// deep-walk setup arms each ganger.
fn weapon_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        WeaponSpec {
            base_spread: BaseSpread::new(0.25),
            accuracy:    Accuracy::new(1.0),
            kickback:    Kickback::new(0.4),
            fatal_bias:  FatalBias::new(7.0),
            damage:      WeaponDamage::new(12),
            punch:       WeaponPunch::new(5),
            shred:       WeaponShred::new(3),
            damage_type: DamageType::Kinetic,
            magazine:    Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
            fire_mode:   FireMode::new(vec![FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.5),
                ModeShots::new(1),
            )]),
            stable:      Stable::new(false),
        },
    )])
}

/// A budget large enough to drive the deep walk down into the battlescape (each
/// leaf scene spends a couple of `FixedUpdate` ticks plus its transition
/// propagation), but bounded so a machine that never reaches the predicate fails
/// instead of hanging.
const BUDGET: u32 = 96;

/// Build a `(cell, level)` key from raw coordinates.
fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// An arbitrary roster armor record (distinct per-part magnitudes, NOT shipped
/// tuning) so a fixture ganger carries a faithful `SourceArmor`.
const fn arbitrary_armor(base: i32) -> SourceArmor {
    SourceArmor::uniform(ArmorPiece::new(
        ArmorFloor::new(base),
        ArmorProtection::new(base + 1),
        ArmorIntegrity::new(base + 2),
        ArmorHardness::new(base + 3),
        ArmorType::DEFAULT,
    ))
}

/// Build an authored ganger at `at` with arbitrary-but-valid component values.
fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawn {
        at,
        name: GangerName::new(format!("Ganger {faction}")),
        faction: Faction::new(faction),
        facing: Facing::new(Direction::East),
        stance: Stance::new(StanceKind::Crouching),
        aiming: Aiming::new(true),
        hp: Hp::new(40),
        hp_max: HpMax::new(40),
        wounds: Wounds::new(3),
        wounds_max: WoundsMax::new(3),
        tu: Tu::new(60),
        tu_max: TuMax::new(60),
        life_state: LifeState::Alive,
        shooting: Shooting::new(f32::from(faction) + 2.0),
        toughness: Toughness::new(f32::from(faction) + 3.0),
        luck: Luck::new(f32::from(faction) + 1.0),
        armor: arbitrary_armor(i32::from(faction) + 1),
        // Every fixture ganger references the one TEST_WEAPON_KEY in weapon_registry.
        weapon: WeaponName::new(TEST_WEAPON_KEY.to_owned()),
    }
}

/// A valid two-ganger fixture situation (no cover / slabs / links needed — a
/// link-free situation validates trivially).
fn two_ganger_situation() -> Situation {
    Situation {
        gangers: vec![ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)],
        ..Situation::new()
    }
}

/// A situation with a DANGLING vertical link (an endpoint at a `(cell, level)` no
/// authored tile occupies) — `setup_battle` returns `Err(DanglingCell)` and inserts
/// no resource (the `setup_aborts_on_invalid_vertical_link` precedent).
fn dangling_link_situation() -> Situation {
    let present = key(4, 4, 0);
    let missing = key(4, 4, 1); // never authored — the link dangles off it
    Situation {
        gangers: vec![ganger_at(key(0, 0, 0), 0)],
        slabs: vec![present], // only `present` authored; `missing` dangles
        vertical_links: vec![VerticalLink::new(present, missing, LinkKind::stair())],
        ..Situation::new()
    }
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
    let mut app = GdtfTestAppBuilder::new().default_start().build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // The Load-built WeaponRegistry (GTW-257): persistent `Load` state the real app
    // resolves from assets/weapons/, injected here for the MinimalPlugins deep-walk
    // (no AssetServer) so the Generation setup arms each ganger from it.
    app.world_mut().insert_resource(weapon_registry());
    // GTW-261: the Load→Intro gate now requires a LoadedSituation; seed the fixture
    // when given, else the empty default so the walk still traverses Load.
    app.world_mut()
        .insert_resource(LoadedSituation(situation.unwrap_or_default()));
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

/// AC2 — entering Generation seeds the battle `SimRng`, and the seed is threaded
/// through `SimRng::from_seed`. The determinism relation (never a pinned magnitude):
/// two `SimRng`s from this slice's same default seed draw EQUAL first `next_u64`s,
/// and a DIFFERENT seed draws a different first value.
#[test]
fn entering_generation_seeds_sim_rng() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach BattleScapeState::Generation within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    assert!(
        app.world().get_resource::<SimRng>().is_some(),
        "entering Generation must insert a SimRng (the battle-lifetime RNG)",
    );

    // Determinism relation: the inserted SimRng's stream must match a SimRng built
    // from the SAME default seed (0) the plugin uses, and differ from a different
    // seed — proving the seed is threaded through SimRng::from_seed, not faked.
    let inserted_first = app
        .world_mut()
        .get_resource_mut::<SimRng>()
        .map(|mut rng| rng.next_u64());
    let mut same_seed = SimRng::from_seed(BattleSeed::new(0));
    let mut diff_seed = SimRng::from_seed(BattleSeed::new(1));
    assert_eq!(
        inserted_first,
        Some(same_seed.next_u64()),
        "the inserted SimRng must draw the same first value as the slice's default seed",
    );
    assert_ne!(
        same_seed.next_u64(),
        diff_seed.next_u64(),
        "a different BattleSeed must yield a different draw — the seed is genuinely threaded",
    );
}

/// AC3 — `setup_battle` runs on the real `Commands` path: its four resources land
/// in the world, and the authored ganger count equals the spawned `WornArmor`
/// entity count — proving the real setup ran, not a stub.
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

    // The spawned ganger count equals the authored count (the WornArmor-count
    // precedent): exactly the fixture's gangers were spawned on the real path.
    let world = app.world_mut();
    let mut query = world.query::<&WornArmor>();
    assert_eq!(
        query.iter(world).count(),
        authored_gangers,
        "the spawned WornArmor ganger count must equal the authored ganger count",
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
        app.world().get_resource::<SimRng>().is_some(),
        "SimRng must survive past Generation (battle-lifetime)",
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
        app.world().get_resource::<SimRng>().is_none(),
        "SimRng must be cleaned on leaving the battle",
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
    let mut query = world.query::<&WornArmor>();
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
