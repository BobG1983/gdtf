//! GTW-236 (lifecycle): the battlescape PERSISTS in `BattleRunning` + the drive proof.
//!
//! `BattleRunning` no longer auto-walks back out: the placeholder 3-tick turn-budget
//! gate (`BattleRunTurnBudget` + its decrement/track systems) is GONE. The battle now
//! RESTS in `BattleScapeState::BattleRunning` indefinitely and leaves ONLY once the
//! explicit end-signal marker `BattleRunningComplete` is inserted — only then does the
//! existing `move_on` advance to `AnimateOut`. The victory census / flee button (sibling
//! slices) are what insert it in the running game; these tests insert it through the
//! `test_support` surface to stand in for that. They are headless `MinimalPlugins` (via
//! [`GdtfTestAppBuilder`]); they bypass the `Load` scene, so each injects the persistent
//! `Load` resources (`GdtfTheme` + `CombatTuning`) the machine needs to traverse `Load`.
//! They are *pin-discriminating*: each assertion re-encodes one acceptance criterion so a
//! regression turns the test red — a re-added auto-exit turns the persistence assertions
//! red.
//!
//! The drive proof (AC1) sets the battle up via the REAL message-driven path: it inserts
//! a [`LoadedSituation`] BEFORE driving, the app's `OnEnter(Generation)` sends
//! `SetupBattleRequested`, and the sim's `setup_battle` spawns the gangers via `Commands`.
//! The test then QUERIES the spawned ganger entities (by faction) off the world — it never
//! takes a `&mut World` in a helper and never hand-spawns an entity that bypasses
//! `setup_battle`. The accepted `gdtf_app` harness idiom — `app.world_mut()` /
//! `app.world()` in the TEST BODY for `insert_resource` / `write_message` / queries — is
//! used throughout (every sibling `gdtf_app` integration test does the same).

use bevy::{ecs::entity::Entity, state::state::State};
use gdtf_app::test_support::{
    BattleRunningComplete, BattleScapeState, LoadedSituation, RunningState,
};
use gdtf_battle_sim::{
    acts::FireRequested,
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        SourceArmor,
    },
    battle::BattleInProgress,
    cover::HeightBand,
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Shooting, Stance, StanceKind,
        Toughness, Tu, TuMax, Wounds,
    },
    magazine::Magazine,
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    situation::{GangerSpawn, Situation},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeShots, ModeTuPercent, Stable,
        WeaponBundle, WeaponDamage, WeaponPunch, WeaponShred,
    },
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk down into the battlescape (each leaf
/// scene spends a couple of `FixedUpdate` ticks plus its transition propagation), but
/// bounded so a machine that never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// The shooter's faction in the drive-proof fixture (the armed ganger).
const SHOOTER_FACTION: u8 = 0;
/// The target's faction in the drive-proof fixture (the ganger fired at).
const TARGET_FACTION: u8 = 1;

/// The `(cell, level)` the shooter is authored at — west of the target, same storey,
/// so a due-East shot reaches it.
const SHOOTER_AT: (i32, i32, u8) = (2, 5, 0);
/// The `(cell, level)` the target is authored at — due East of the shooter.
const TARGET_AT: (i32, i32, u8) = (8, 5, 0);

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

/// Build an authored ganger at `at` with arbitrary-but-valid component values. The
/// target carries paper-thin armor (`base 0`) so a landed shot lands in a known regime.
fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawn {
        at,
        faction: Faction::new(faction),
        facing: Facing::new(Direction::East),
        stance: Stance::new(StanceKind::Standing),
        aiming: Aiming::new(true),
        hp: Hp::new(40),
        wounds: Wounds::new(3),
        tu: Tu::new(60),
        life_state: LifeState::Alive,
        shooting: Shooting::new(f32::from(faction) + 2.0),
        toughness: Toughness::new(f32::from(faction) + 3.0),
        luck: Luck::new(f32::from(faction) + 1.0),
        armor: arbitrary_armor(i32::from(faction)),
    }
}

/// A valid two-ganger fixture situation (link-free → validates trivially): an armed-able
/// shooter (faction `SHOOTER_FACTION`) facing East, and a target (faction `TARGET_FACTION`)
/// directly East. The `SetupBattleRequested` the app sends on `OnEnter(Generation)` pours
/// this real battle into the world before `BattleRunning` (`SimRng` / `CombatTuning` /
/// `OccupancyGrid` present, both gangers spawned by `setup_battle` via `Commands`).
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
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// The weapon-state bundle a setup-spawned ganger LACKS (a `GangerSpawn` authors no
/// weapon — E10.3 added only armor authoring), assembled so the drive proof can ARM a
/// real, queried-from-`setup_battle` shooter by `insert`-ing these onto its existing
/// entity (the `acts.rs` AC7 `entity_mut(..).insert(..)` precedent — augmenting an
/// already-spawned entity, NOT spawning a new one). Arbitrary magnitudes (not shipped
/// tuning). Mirrors `acts.rs::spawn_shooter`'s weapon kit.
fn shooter_weapon_kit(mode: FireModeSpec) -> impl bevy::prelude::Bundle {
    let mag_size = MagazineSize::new(30);
    (
        WeaponBundle::new(
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
            HandlingProfile::new(
                mag_size,
                FireMode::Single { single: mode },
                Stable::new(true),
            ),
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

/// Builds the headless walk app, injecting the persistent `Load` resources the machine
/// needs to traverse `Load` (no `AssetServer` under `MinimalPlugins`), plus an optional
/// `LoadedSituation` fixture for the Generation setup to pour into the world.
fn walk_app(situation: Option<Situation>) -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new().default_start().build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    if let Some(situation) = situation {
        app.world_mut().insert_resource(LoadedSituation(situation));
    }
    app
}

/// Drives the app from the default start down to the first update on which
/// [`BattleScapeState::BattleRunning`] is active. Returns whether it was reached.
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

/// AC1 — DRIVE PROOF: a `FireRequested` emitted while in `BattleRunning` drives the sim
/// (the E10.5-bundled, witness-gated dispatch band is live in `BattleRunning`).
///
/// Sets the battle up via the REAL message-driven path: inserts a two-ganger
/// `LoadedSituation`, drives to `BattleRunning` (the app's `OnEnter(Generation)` sends
/// `SetupBattleRequested`, the sim's `setup_battle` spawns both gangers via `Commands`),
/// then QUERIES the spawned shooter + target entities by faction. It ARMS the queried
/// shooter by `insert`-ing the weapon kit a `GangerSpawn` does not author (the
/// `entity_mut(..).insert(..)` augment-an-existing-entity precedent), PUBLISHES the
/// target's occupant band in the live `OccupancyGrid` (the silhouette the band-free march
/// reads to resolve a `Ganger` hit), captures the target's baseline, emits a
/// `FireRequested` inline via the world message buffer, `update()`s once, and asserts ≥1
/// queried target component (`Hp`/`Wounds`/`LifeState`) changed OR the shooter's `Tu`
/// dropped (a seed-dependent miss still charges TU) — a relation, never a pinned
/// magnitude. No `&mut World` helper; no hand-spawn that bypasses `setup_battle`.
#[test]
fn fire_requested_in_battle_running_drives_the_sim() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleScapeState::BattleRunning within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // The battle was poured into the world by the real Generation setup — the witness the
    // bundled dispatch band gates on (GTW-212's BattleInProgress, no longer OccupancyGrid)
    // is present, and the OccupancyGrid the march reads is too.
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "a battle must be set up in BattleRunning (BattleInProgress witness present)",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "the OccupancyGrid the band-free march reads must be present in BattleRunning",
    );

    // Query the two SETUP-SPAWNED gangers (never hand-spawned): the shooter (faction 0)
    // and the target (faction 1). The real setup must have spawned exactly one of each.
    let shooter_found = find_ganger(&mut app, SHOOTER_FACTION);
    assert!(
        shooter_found.is_some(),
        "the real setup must have spawned a faction-{SHOOTER_FACTION} shooter ganger",
    );
    let target_found = find_ganger(&mut app, TARGET_FACTION);
    assert!(
        target_found.is_some(),
        "the real setup must have spawned a faction-{TARGET_FACTION} target ganger",
    );
    let (Some(shooter), Some(target)) = (shooter_found, target_found) else {
        return;
    };
    assert_ne!(
        shooter, target,
        "the shooter and target are distinct entities"
    );

    // ARM the queried shooter — a `GangerSpawn` authors no weapon (E10.3 added only armor),
    // so `insert` the weapon kit onto the EXISTING setup-spawned entity (augment, never
    // re-spawn). This is the only way the bundled `dispatch_fire` (which needs
    // `With<Weapon>` + every weapon stat) resolves a real shot from a setup-spawned shooter.
    let mode = single_mode(0.2, 1);
    app.world_mut()
        .entity_mut(shooter)
        .insert(shooter_weapon_kit(mode));

    // PUBLISH the target's occupant band in the live grid — the band-free march reads it
    // to band the round vs the occupant (a `Ganger` hit needs both occupant + band, and
    // neither `setup_battle` nor the maintenance layer publishes the band). HIGH so a
    // standing target is squarely in the round's path.
    let (tx, ty, tl) = TARGET_AT;
    let target_at = key(tx, ty, tl);
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }

    let hp_before = app.world().get::<Hp>(target).copied();
    let wounds_before = app.world().get::<Wounds>(target).map(|w| **w);
    let life_before = app.world().get::<LifeState>(target).copied();
    let tu_before = app.world().get::<Tu>(shooter).copied();

    // Emit the fire request IN BattleRunning, then advance one update so the bundled
    // dispatch band (live battle-wide) consumes it.
    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        Cell::new(tx, ty),
        Level::new(tl),
    ));
    app.update();

    let hp_after = app.world().get::<Hp>(target).copied();
    let wounds_after = app.world().get::<Wounds>(target).map(|w| **w);
    let life_after = app.world().get::<LifeState>(target).copied();
    let tu_after = app.world().get::<Tu>(shooter).copied();

    let target_changed =
        hp_after != hp_before || wounds_after != wounds_before || life_after != life_before;
    let tu_dropped = matches!((tu_before, tu_after), (Some(b), Some(a)) if *a < *b);
    assert!(
        target_changed || tu_dropped,
        "a FireRequested in BattleRunning must drive the sim — a target component changed or the \
         shooter's Tu dropped (hp {hp_before:?}->{hp_after:?}, wounds {wounds_before:?}->\
         {wounds_after:?}, life {life_before:?}->{life_after:?}, tu {tu_before:?}->{tu_after:?})",
    );
}

/// The persistence stress count — N ≫ 3 (the deleted placeholder budget was 3 ticks),
/// so a re-added auto-exit (which fired within ~3 `FixedUpdate` ticks) would advance OFF
/// `BattleRunning` well within this loop and turn the persistence assertion red.
const PERSIST_UPDATES: u32 = 32;

/// AC1/AC2 — PERSISTENCE: with no end-signal marker inserted, `BattleRunning` PERSISTS
/// across N ≫ 3 (`PERSIST_UPDATES`) updates — it never reaches `AnimateOut`/`AfterMath`.
///
/// This is the core GTW-236 change: the placeholder 3-tick turn-budget auto-exit is gone, so
/// no budget/timer/tick system inserts `BattleRunningComplete` on its own. A regression that
/// re-adds an auto-insert would trip `move_on` (advancing to `AnimateOut`) within ~3 ticks,
/// failing the per-iteration assert.
#[test]
fn battle_running_persists_without_an_end_signal() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleRunning within {BUDGET} updates; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // Across many updates with NO explicit marker, the machine stays put in BattleRunning.
    for iteration in 0..PERSIST_UPDATES {
        app.update();
        assert_eq!(
            battlescape_state(&app),
            Some(BattleScapeState::BattleRunning),
            "BattleRunning must PERSIST with no BattleRunningComplete inserted (GTW-236) — it must \
             not auto-advance to AnimateOut; failed on update {iteration} of {PERSIST_UPDATES}",
        );
    }
}

/// AC3 — the explicit end still works: while resting in `BattleRunning`, inserting
/// `BattleRunningComplete` (through the `test_support` surface, standing in for the
/// not-yet-wired victory/flee end condition) then `update()`-ing advances `BattleScapeState`
/// to `AnimateOut` — proving the marker-gated `move_on` registration survived the rework.
#[test]
fn explicit_end_marker_advances_to_animate_out() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleRunning within {BUDGET} updates; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // Insert the explicit end-signal marker (the seam victory/flee will drive).
    app.world_mut().insert_resource(BattleRunningComplete);

    let reached_animate_out = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::AnimateOut),
        BUDGET,
    );
    assert!(
        reached_animate_out,
        "an explicit BattleRunningComplete insert must trip move_on and advance BattleRunning → \
         AnimateOut within {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
}
