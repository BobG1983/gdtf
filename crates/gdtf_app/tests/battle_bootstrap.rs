//! GTW-209 (E10.7): the headless end-to-end bootstrap CAPSTONE — a battle
//! constructs AND runs end-to-end under `MinimalPlugins`, with NO production code
//! of its own. It consumes ONLY what the prior E10 slices landed: the E10.1
//! `gdtf_app → gdtf_battle_sim` Cargo edge (so this crate can name sim types), the
//! E10.0 [`SimSystems::Simulate`] band, the E10.2 `*Requested` message contract +
//! per-act dispatch, the E10.5 `BattleSimPlugin` (Generation seeds the [`SimRng`]
//! from the chosen [`BattleSeed`] source + inserts [`CombatTuning`] + runs
//! `setup_battle`), and the GTW-212 [`BattleInProgress`]-gated battle-wide dispatch
//! that E10.6 keeps live across `BattleRunning`.
//!
//! Built on [`GdtfTestAppBuilder`] (`MinimalPlugins`, a one-tick fixed timestep, and the
//! real `ScenesPlugin` state machine) exactly as `state_walk.rs` and the E10.6
//! `battle_running_driver.rs` are. It seeds the resources the real `Load` scene resolves
//! from assets but a `MinimalPlugins` app has no `AssetServer` to load
//! ([`GdtfTheme`](gdtf_ui::theme::GdtfTheme) via [`default_theme`], plus [`CombatTuning`]),
//! inserts the test's authored [`Situation`] inline as a [`LoadedSituation`] (the
//! resolved-load stand-in the E10.6 precedent established — `Situation` derives
//! `Deserialize` now, E10.3/GTW-205, but the test owns the fixture inline), drives the
//! state machine down to `BattleScapeState::BattleRunning`, and exercises the REAL path
//! end-to-end (real `ScenesPlugin`, real `setup_battle`, real E10.2 dispatch, real verb,
//! the injected seeded [`SimRng`]).
//!
//! These are *pin-discriminating* tests: each `#[test]` re-encodes one acceptance
//! criterion as a before≠after / equality RELATION (never a pinned tunable magnitude),
//! so a regression in the E10 wiring turns the test red.
//!
//! CORRECTIONS honored (the contract's CORRECTIONS block):
//! 1. The test inserts a [`LoadedSituation`] + drives; it does NOT manually send
//!    `SetupBattleRequested` (the app's `OnEnter(Generation)` does).
//! 2. A `GangerSpawn` authors NO weapon, so the drive proof ARMS the queried shooter in
//!    the TEST BODY via `app.world_mut().entity_mut(shooter).insert(<kit>)` and PUBLISHES
//!    the target's occupant band in the live [`OccupancyGrid`] (the E10.6 precedent).
//! 3. Dispatch runs BATTLE-WIDE via the GTW-212 [`BattleInProgress`]-gated band, present
//!    across `BattleRunning`, so a `*Requested` emitted in `BattleRunning` resolves.
//!
//! NO function in this file takes `&mut World`/`&World`; every `app.world_mut()` /
//! `app.world()` call is in the TEST BODY (the established `gdtf_app` test idiom). No
//! `Camera`, no `Window`, no `gdtf_battle_presenter`, no `gdtf_battle_input`, no
//! `*Resolved` type, and the drive STOPS at `BattleScapeState::BattleRunning` (never
//! `AfterMath` / `AfterMathState`).

use bevy::{ecs::entity::Entity, state::state::State};
use gdtf_app::test_support::{
    AppState, BattleScapeState, GameState, LoadedSituation, RunningState,
};
use gdtf_battle_sim::{
    acts::{FireRequested, SetStanceRequested},
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        SourceArmor,
    },
    battle::BattleInProgress,
    cover::{CoverLedger, HeightBand},
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Shooting, Stance, StanceKind,
        Toughness, Tu, TuMax, Wounds,
    },
    magazine::Magazine,
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    rng::SimRng,
    situation::{GangerSpawn, Situation},
    surface::SurfaceGrid,
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry, WeaponShred,
        WeaponSpec,
    },
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// The weapon KEY every fixture ganger references — present in [`weapon_registry`]
/// (the same `"test-weapon"` key the deterministic [`shooter_weapon_kit`] re-arms with).
const TEST_WEAPON_KEY: &str = "test-weapon";

/// A registry holding the one [`TEST_WEAPON_KEY`] weapon the fixture gangers
/// reference, standing in for the `Load`-built registry (GTW-257) so the deep-walk
/// setup arms each ganger. The drive proof later OVERWRITES the shooter's weapon with
/// the deterministic [`shooter_weapon_kit`]; the registry only needs the key to exist.
fn weapon_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        WeaponSpec {
            base_spread:   BaseSpread::new(0.25),
            accuracy:      Accuracy::new(1.0),
            kickback:      Kickback::new(0.4),
            fatal_bias:    FatalBias::new(0.0),
            damage:        WeaponDamage::new(12),
            punch:         WeaponPunch::new(5),
            shred:         WeaponShred::new(3),
            damage_type:   DamageType::Kinetic,
            magazine_size: MagazineSize::new(30),
            fire_mode:     FireMode::new(vec![FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.5),
                ModeShots::new(1),
            )]),
            stable:        Stable::new(false),
        },
    )])
}

/// A budget large enough to drive the deep walk down into the battlescape (each leaf
/// scene spends a couple of `FixedUpdate` ticks plus its transition propagation), but
/// bounded so a machine that never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// The shooter's faction in the fixture (the ganger the drive proof arms + fires).
const SHOOTER_FACTION: u8 = 0;
/// The target's faction in the fixture (the ganger fired at).
const TARGET_FACTION: u8 = 1;

/// The `(cell, level)` the shooter is authored at — west of the target, same storey,
/// so a due-East shot reaches it.
const SHOOTER_AT: (i32, i32, u8) = (2, 5, 0);
/// The `(cell, level)` the target is authored at — due East of the shooter, close
/// range so the shot lands deterministically under the fixed seed.
const TARGET_AT: (i32, i32, u8) = (8, 5, 0);

/// The stance every fixture ganger is authored holding — the AC3 "authored start" value
/// the requested stance must differ from.
const AUTHORED_STANCE: StanceKind = StanceKind::Standing;
/// The stance the AC3 `SetStanceRequested` asks for — DISTINCT from [`AUTHORED_STANCE`],
/// so a successful flip is observable as a change to exactly this value.
const REQUESTED_STANCE: StanceKind = StanceKind::Prone;

/// Build a `(cell, level)` key from raw coordinates.
fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// An arbitrary roster armor record (distinct per-part magnitudes, NOT shipped tuning).
const fn arbitrary_armor(base: i32) -> SourceArmor {
    SourceArmor::uniform(ArmorPiece::new(
        ArmorFloor::new(base),
        ArmorProtection::new(base + 1),
        ArmorIntegrity::new(base + 2),
        ArmorHardness::new(base + 3),
        ArmorType::DEFAULT,
    ))
}

/// Build an authored ganger at `at` with arbitrary-but-valid component values, holding
/// the [`AUTHORED_STANCE`]. The target carries paper-thin armor (`base 0`) so a landed
/// shot lands in a known regime.
fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawn {
        at,
        faction: Faction::new(faction),
        facing: Facing::new(Direction::East),
        stance: Stance::new(AUTHORED_STANCE),
        aiming: Aiming::new(true),
        hp: Hp::new(40),
        wounds: Wounds::new(3),
        tu: Tu::new(60),
        tu_max: TuMax::new(60),
        life_state: LifeState::Alive,
        shooting: Shooting::new(f32::from(faction) + 2.0),
        toughness: Toughness::new(f32::from(faction) + 3.0),
        luck: Luck::new(f32::from(faction) + 1.0),
        armor: arbitrary_armor(i32::from(faction)),
        // Every fixture ganger references the one TEST_WEAPON_KEY in weapon_registry.
        weapon: WeaponName::new(TEST_WEAPON_KEY.to_owned()),
    }
}

/// The authored ganger count the fixture spawns — the AC1 `WornArmor`-count assertion
/// reads this exact number.
const AUTHORED_GANGER_COUNT: usize = 2;

/// A valid two-ganger fixture situation (link-free → validates trivially): a shooter
/// (faction [`SHOOTER_FACTION`]) facing East, and a target (faction [`TARGET_FACTION`])
/// directly East at close range. The `SetupBattleRequested` the app sends on
/// `OnEnter(Generation)` pours this real battle into the world before `BattleRunning`.
fn two_ganger_situation() -> Situation {
    let (sx, sy, sl) = SHOOTER_AT;
    let (tx, ty, tl) = TARGET_AT;
    Situation {
        gangers: vec![
            ganger_at(key(sx, sy, sl), SHOOTER_FACTION),
            ganger_at(key(tx, ty, tl), TARGET_FACTION),
        ],
        ..Situation::new()
    }
}

/// A single-shot fire-mode spec from arbitrary (non-pinned) per-mode numbers — the
/// `acts.rs` fire-test precedent.
const fn single_mode(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// The DETERMINISTIC weapon-state bundle the drive proof RE-ARMS the shooter with —
/// since GTW-257 `setup_battle` arms every ganger from the registry, this OVERWRITES
/// that registry weapon (via a second `insert` on the queried-from-`setup_battle`
/// shooter entity) with a precise, high-damage kit so the test's single shot lands in a
/// known regime. It ALSO supplies `TuMax` + `Magazine`, which `setup_battle` does NOT
/// add (the `WeaponBundle` carries only the `MagazineSize` capacity). Arbitrary
/// magnitudes (not shipped tuning). Mirrors the `acts.rs` weapon kit.
fn shooter_weapon_kit(mode: FireModeSpec) -> impl bevy::prelude::Bundle {
    let mag_size = MagazineSize::new(30);
    (
        WeaponBundle::new(
            WeaponName::new(String::from("test-weapon")),
            BaseSpread::new(0.05),
            Accuracy::new(2.0),
            Kickback::new(0.2),
            FatalBias::new(0.0),
            DamageProfile::new(
                WeaponDamage::new(40),
                WeaponPunch::new(20),
                WeaponShred::new(10),
                DamageType::Kinetic,
            ),
            HandlingProfile::new(mag_size, FireMode::new(vec![mode]), Stable::new(true)),
        ),
        // The shooter query also reads TuMax and decrements a Magazine — neither is
        // authored by `setup_battle`, so the kit supplies both.
        TuMax::new(100),
        Magazine::new(10, mag_size),
    )
}

/// Find the spawned ganger entity of `faction` in the world (the real-path query the
/// drive proof uses instead of hand-spawning). Returns the FIRST match — the fixture
/// authors exactly one ganger per faction. Runs entirely off `app.world_mut()` (the
/// accepted test-body idiom): it spawns nothing and takes no `&mut World` helper param.
fn find_ganger(app: &mut bevy::app::App, faction: u8) -> Option<Entity> {
    let wanted = Faction::new(faction);
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find_map(|(entity, &f)| (f == wanted).then_some(entity))
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`GameState`] if it is active.
fn game_state(app: &bevy::app::App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds the headless capstone app, starting at [`AppState::Running`] and seeding the
/// persistent `Load` resources the machine needs to traverse `Load` under
/// `MinimalPlugins` (no `AssetServer`): [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) via
/// [`default_theme`] + [`CombatTuning`] (the `state_walk` / E10.6 precedent), plus the
/// test's authored [`Situation`] inserted inline as a [`LoadedSituation`] for the
/// Generation setup to pour into the world.
fn capstone_app(situation: Situation) -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // The Load-built WeaponRegistry (GTW-257) so the Generation setup arms each ganger.
    app.world_mut().insert_resource(weapon_registry());
    app.world_mut().insert_resource(LoadedSituation(situation));
    app
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

/// Drives the capstone app from the [`AppState::Running`] start down to the first update
/// on which [`BattleScapeState::BattleRunning`] is active. Returns whether it was reached.
fn drive_to_battle_running(app: &mut bevy::app::App) -> bool {
    if !drive_past_menu(app) {
        return false;
    }
    advance_until(
        app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    )
}

/// AC1 — Bootstrap reaches `BattleScape` with the sim constructed. Driving the capstone
/// (seeded with [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) + [`CombatTuning`] + the inline
/// [`Situation`] source, started at [`AppState::Running`], driven past
/// [`RunningState::Menu`]) descends to [`GameState::BattleScape`], and once
/// [`BattleScapeState::Generation`] has run E10.5's wired `setup_battle` the world holds
/// the four sim resources ([`OccupancyGrid`] / [`CoverLedger`] / [`SurfaceGrid`] /
/// [`VerticalLinkGraph`]), the seeded [`SimRng`], AND the [`CombatTuning`] present
/// through the battle — and `query::<&WornArmor>().count()` equals the authored ganger
/// count (the `setup_battle` C8(a) entity-count precedent, proving the entities were
/// spawned by the REAL setup, not a no-op scaffold).
#[test]
fn bootstrap_reaches_battlescape_with_the_sim_constructed() {
    let mut app = capstone_app(two_ganger_situation());
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should descend to BattleScapeState::BattleRunning within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // We descended through GameState::BattleScape (the Generation child ran on the way).
    assert_eq!(
        game_state(&app),
        Some(GameState::BattleScape),
        "the walk must rest inside GameState::BattleScape after reaching BattleRunning",
    );

    // The four sim resources E10.5's wired setup_battle inserts during Generation.
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "Generation's setup_battle must insert an OccupancyGrid",
    );
    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "Generation's setup_battle must insert a CoverLedger",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "Generation's setup_battle must insert a SurfaceGrid",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "Generation's setup_battle must insert a VerticalLinkGraph",
    );
    // The seeded SimRng E10.5 builds from the chosen BattleSeed source during Generation.
    assert!(
        app.world().get_resource::<SimRng>().is_some(),
        "Generation must insert the seeded SimRng",
    );
    // CombatTuning is present through the battle (E10.4's persistent Load resource).
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "CombatTuning must be present in BattleRunning",
    );

    // The authored gangers were spawned as entities by the REAL setup_battle: exactly
    // one WornArmor-carrying entity per authored ganger (count-equality, not a magnitude).
    let world = app.world_mut();
    let mut worn = world.query::<&gdtf_battle_sim::armor::WornArmor>();
    assert_eq!(
        worn.iter(world).count(),
        AUTHORED_GANGER_COUNT,
        "the world must hold exactly the authored ganger count of WornArmor entities — the real \
         setup_battle spawned the gangers, not a no-op scaffold",
    );
}

/// AC2 — A `FireRequested` emitted inline in `BattleRunning` mutates the model. With the
/// app rested in [`BattleScapeState::BattleRunning`], the drive proof QUERIES the spawned
/// shooter + target by [`Faction`] (off `app.world_mut()`, the test-body idiom), ARMS the
/// queried shooter via `entity_mut(..).insert(<weapon kit + TuMax + Magazine>)` (a
/// `GangerSpawn` authors no weapon), PUBLISHES the target's occupant band in the live
/// [`OccupancyGrid`] (the silhouette the band-free march reads to resolve a `Ganger`
/// hit), snapshots the target's `Hp`/`Wounds`/`LifeState`, emits a [`FireRequested`]
/// inline via the world message buffer, `update()`s once so the GTW-212-gated battle-wide
/// dispatch consumes it, and asserts ≥1 of `Hp`/`Wounds`/`LifeState` changed — a landed
/// hit, phrased as a before≠after relation, never a pinned magnitude.
#[test]
fn fire_requested_in_battle_running_mutates_the_model() {
    let mut app = capstone_app(two_ganger_situation());
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleScapeState::BattleRunning within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // The witness the GTW-212-gated battle-wide dispatch keys on is present in
    // BattleRunning, and the OccupancyGrid the march reads is too.
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "the BattleInProgress witness must be present in BattleRunning (the dispatch gate)",
    );

    // Query the two SETUP-SPAWNED gangers (never hand-spawned): the shooter + the target.
    let shooter_found = find_ganger(&mut app, SHOOTER_FACTION);
    let target_found = find_ganger(&mut app, TARGET_FACTION);
    assert!(
        shooter_found.is_some(),
        "the real setup must have spawned a faction-{SHOOTER_FACTION} shooter ganger",
    );
    assert!(
        target_found.is_some(),
        "the real setup must have spawned a faction-{TARGET_FACTION} target ganger",
    );
    let (Some(shooter), Some(target)) = (shooter_found, target_found) else {
        return;
    };
    assert_ne!(shooter, target, "shooter and target are distinct entities");

    // ARM the queried shooter — a GangerSpawn authors no weapon, so insert the kit onto
    // the EXISTING setup-spawned entity (augment, never re-spawn).
    let mode = single_mode(0.2, 1);
    app.world_mut()
        .entity_mut(shooter)
        .insert(shooter_weapon_kit(mode));

    // PUBLISH the target's occupant band in the live grid — the band-free march reads it
    // to band the round vs the occupant. HIGH so a standing target is squarely in path.
    let (tx, ty, tl) = TARGET_AT;
    let target_at = key(tx, ty, tl);
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }

    // Snapshot the target's battle surfaces before the emit.
    let hp_before = app.world().get::<Hp>(target).copied();
    let wounds_before = app.world().get::<Wounds>(target).copied();
    let life_before = app.world().get::<LifeState>(target).copied();

    // Emit the fire request IN BattleRunning, then advance one update so the gated
    // battle-wide dispatch (live across the battle) consumes it and runs fire().
    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        Cell::new(tx, ty),
        Level::new(tl),
    ));
    app.update();

    let hp_after = app.world().get::<Hp>(target).copied();
    let wounds_after = app.world().get::<Wounds>(target).copied();
    let life_after = app.world().get::<LifeState>(target).copied();

    let target_changed =
        hp_after != hp_before || wounds_after != wounds_before || life_after != life_before;
    assert!(
        target_changed,
        "a FireRequested in BattleRunning must land a hit — a target component changed (hp \
         {hp_before:?}->{hp_after:?}, wounds {wounds_before:?}->{wounds_after:?}, life \
         {life_before:?}->{life_after:?})",
    );
}

/// AC3 — A `SetStanceRequested` flips its addressed component through the dispatch
/// boundary. Emitting a [`SetStanceRequested`] inline in `BattleRunning`, carrying the
/// actor [`Entity`] + a [`REQUESTED_STANCE`] DISTINCT from the [`AUTHORED_STANCE`] start,
/// then `update()`, mutates exactly that [`Stance`] on exactly that entity through the
/// GTW-212-gated, E10.2-owned dispatch + the landed `set_stance` verb. The test reads the
/// actor's [`Stance`] before and after, asserting it differs from the authored start
/// before and equals the requested value after — proving the message→dispatch→verb path
/// carries the actor [`Entity`] correctly (a relation, no tunable pinned).
#[test]
fn set_stance_requested_in_battle_running_flips_the_component() {
    let mut app = capstone_app(two_ganger_situation());
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleScapeState::BattleRunning within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // Address the setup-spawned shooter as the posture actor (it carries Stance + Tu).
    let actor_found = find_ganger(&mut app, SHOOTER_FACTION);
    assert!(
        actor_found.is_some(),
        "the real setup must have spawned a faction-{SHOOTER_FACTION} shooter ganger",
    );
    let Some(actor) = actor_found else {
        return;
    };

    // The authored start stance is the distinct baseline the flip must move off of.
    let stance_before = app.world().get::<Stance>(actor).map(|s| **s);
    assert_eq!(
        stance_before,
        Some(AUTHORED_STANCE),
        "the spawned actor must hold the authored start stance before the request",
    );

    // Emit a stance request for a DISTINCT stance, then advance one update so the gated
    // dispatch consumes it and runs set_stance.
    app.world_mut()
        .write_message(SetStanceRequested::new(actor, REQUESTED_STANCE));
    app.update();

    let stance_after = app.world().get::<Stance>(actor).map(|s| **s);
    assert_eq!(
        stance_after,
        Some(REQUESTED_STANCE),
        "the SetStanceRequested must flip exactly the actor's Stance to the requested value \
         through the dispatch boundary (was {stance_before:?})",
    );
}

/// AC4 — The boundary holds: no panic and seeded determinism. The whole drive
/// (Generation `setup_battle` + a `BattleRunning` `FireRequested` round) runs without
/// panic, and is reproducible: two independent capstone apps built from the SAME inline
/// [`Situation`] and the SAME fixed [`BattleSeed`] source (the app's Generation seeds
/// `SimRng::from_seed` with its fixed `DEFAULT_BATTLE_SEED`, identical across runs),
/// driven through the identical sequence, produce the identical observable outcome. The
/// test builds-and-drives twice and asserts the post-fire target `(Hp, Wounds,
/// LifeState)` tuple is equal across the two runs; the run completing the full
/// descend+emit+assert sequence without aborting is the no-panic evidence.
#[test]
fn the_drive_is_panic_free_and_seed_deterministic_across_runs() {
    // Run the full descend + arm + fire sequence once and snapshot the post-fire target
    // (Hp, Wounds, LifeState) tuple (all Copy + PartialEq). Returns None if the drive did
    // not reach BattleRunning or the setup did not spawn the gangers — the no-unwrap
    // let-else style so the test body stays panic-free.
    let post_fire_target_state = || -> Option<(Option<Hp>, Option<Wounds>, Option<LifeState>)> {
        let mut app = capstone_app(two_ganger_situation());
        if !drive_to_battle_running(&mut app) {
            return None;
        }
        let shooter = find_ganger(&mut app, SHOOTER_FACTION)?;
        let target = find_ganger(&mut app, TARGET_FACTION)?;

        let mode = single_mode(0.2, 1);
        app.world_mut()
            .entity_mut(shooter)
            .insert(shooter_weapon_kit(mode));
        let (tx, ty, tl) = TARGET_AT;
        let target_at = key(tx, ty, tl);
        if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
            grid.set_occupant_band(target_at, Some(HeightBand::High));
        }

        app.world_mut().write_message(FireRequested::new(
            shooter,
            mode,
            Cell::new(tx, ty),
            Level::new(tl),
        ));
        app.update();

        Some((
            app.world().get::<Hp>(target).copied(),
            app.world().get::<Wounds>(target).copied(),
            app.world().get::<LifeState>(target).copied(),
        ))
    };

    let first = post_fire_target_state();
    let second = post_fire_target_state();
    assert!(
        first.is_some(),
        "the seeded drive must reach BattleRunning and fire within {BUDGET} updates",
    );
    assert_eq!(
        first, second,
        "the post-fire (Hp, Wounds, LifeState) tuple must be identical across two fixed-seed runs \
         (got {first:?} vs {second:?}) — the same BattleSeed source reproduces the same outcome",
    );
}

/// AC5 — Strictly headless, no out-of-slice edge, no path past `BattleRunning`. This test
/// re-encodes the headless contract behaviorally: the drive STOPS at
/// [`BattleScapeState::BattleRunning`] and the test never advances past it into
/// [`BattleScapeState::AfterMath`]. (The file's import set names no `Camera`, no
/// `Window`, no `gdtf_battle_presenter`, no `gdtf_battle_input`, no `*Resolved` type, and
/// no `AfterMath` variant — verified by inspection at gate, as `state_walk.rs`'s headless
/// contract is.)
#[test]
fn drive_is_headless_and_stops_at_battle_running() {
    let mut app = capstone_app(two_ganger_situation());
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleScapeState::BattleRunning within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // The drive rests AT BattleRunning — this slice's path stops here and does not enter
    // AfterMath (the AfterMath leg is intentionally out of this slice).
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::BattleRunning),
        "the headless drive must stop at BattleScapeState::BattleRunning, never advancing into \
         AfterMath",
    );
}
