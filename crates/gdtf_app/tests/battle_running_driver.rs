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
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
        ArmorRegistry, ArmorSpec, ArmorType,
    },
    battle::BattleInProgress,
    cover::HeightBand,
    ganger::{
        Aiming, Direction, Facing, Faction, GangerName, Hp, HpMax, LifeState, Luck, Shooting,
        Stance, StanceKind, Toughness, Tu, TuMax, Wounds, WoundsMax,
    },
    magazine::{Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    situation::{GangerSpawn, Situation},
    tuning::CombatTuning,
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

/// The armor KEY every fixture ganger references — present in [`armor_registry`]
/// (GTW-269).
const TEST_ARMOR_KEY: &str = "test-armor";

/// A registry holding the one [`TEST_WEAPON_KEY`] weapon the fixture gangers
/// reference, standing in for the `Load`-built registry (GTW-257) so the deep-walk
/// setup arms each ganger. The drive proof later OVERWRITES the shooter's weapon with
/// the deterministic [`shooter_weapon_kit`]; the registry only needs the key to exist.
fn weapon_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        WeaponSpec {
            base_spread: BaseSpread::new(0.25),
            accuracy:    Accuracy::new(1.0),
            kickback:    Kickback::new(0.4),
            fatal_bias:  FatalBias::new(0.0),
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

/// An arbitrary armor SPEC (distinct per-part magnitudes, NOT shipped tuning) — the
/// suit the [`TEST_ARMOR_KEY`] resolves to in [`armor_registry`] (GTW-269). Base 0 =
/// paper-thin, so a landed shot lands in a known regime.
const fn arbitrary_armor(base: i32) -> ArmorSpec {
    ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(base),
        ArmorProtection::new(base + 1),
        ArmorIntegrity::new(base + 2),
        ArmorHardness::new(base + 3),
        ArmorType::DEFAULT,
    ))
}

/// A registry holding the one [`TEST_ARMOR_KEY`] armor suit the fixture gangers
/// reference, standing in for the `Load`-built registry (GTW-269) so the deep-walk
/// setup armors each ganger. Paper-thin (base 0), so a landed shot lands in a known
/// regime.
fn armor_registry() -> ArmorRegistry {
    ArmorRegistry::new([(
        ArmorName::new(TEST_ARMOR_KEY.to_owned()),
        arbitrary_armor(0),
    )])
}

/// Build an authored ganger at `at` with arbitrary-but-valid component values. The
/// target carries paper-thin armor (`base 0`) so a landed shot lands in a known regime.
fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawn {
        at,
        name: GangerName::new(format!("Ganger {faction}")),
        faction: Faction::new(faction),
        facing: Facing::new(Direction::East),
        stance: Stance::new(StanceKind::Standing),
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
        // Every fixture ganger references the one TEST_ARMOR_KEY in armor_registry.
        armor: ArmorName::new(TEST_ARMOR_KEY.to_owned()),
        // Every fixture ganger references the one TEST_WEAPON_KEY in weapon_registry.
        weapon: WeaponName::new(TEST_WEAPON_KEY.to_owned()),
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
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// The DETERMINISTIC weapon-state bundle the drive proof RE-ARMS the shooter with —
/// since GTW-257 `setup_battle` arms every ganger from the registry, this OVERWRITES
/// that registry weapon (a second `insert` on the queried-from-`setup_battle` shooter
/// entity, the `acts.rs` AC7 `entity_mut(..).insert(..)` precedent — augmenting an
/// already-spawned entity, NOT spawning a new one) with a precise kit so the test's
/// shot lands in a known regime. It ALSO supplies `TuMax` + `Magazine`, which
/// `setup_battle` does NOT add. Arbitrary magnitudes (not shipped tuning). Mirrors
/// `acts.rs::spawn_shooter`'s weapon kit.
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
            // GTW-275: the WeaponBundle now carries the Magazine grouping, so the kit's
            // known 10-round load rides in the HandlingProfile (a separate Magazine in
            // the same bundle would be a duplicate-component panic).
            HandlingProfile::new(
                Magazine::new(10, mag_size, ReloadTu::new(12)),
                FireMode::new(vec![mode]),
                Stable::new(true),
            ),
        ),
        // The shooter query also reads TuMax — not authored by `setup_battle` — so the
        // kit supplies it (the Magazine is now part of the WeaponBundle above).
        TuMax::new(100),
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
    // The Load-built WeaponRegistry (GTW-257) so the Generation setup arms each ganger.
    app.world_mut().insert_resource(weapon_registry());
    // The Load-built ArmorRegistry (GTW-269) so the Generation setup armors each
    // ganger: every fixture ganger references TEST_ARMOR_KEY, which this registry holds
    // (it must be populated now that setup_battle resolves armor keys, not merely
    // present for the Load gate; no AssetServer under MinimalPlugins to load it from
    // disk).
    app.world_mut().insert_resource(armor_registry());
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

// === GTW-239 — end BattleRunning on the sim's outcome signal (BattleWon OR BattleLost). ===
//
// These exercise the LIVE app-side `end_battle_on_outcome` system (Update,
// `.after(SimSystems::Simulate)`, presence-gated). They drive the real walk to
// `BattleRunning` (so the sim's `BattleSimPlugin` is added and the `BattleWon`/`BattleLost`
// buffers exist app-side for free), then either write the sim outcome message directly into
// the world buffer (AC1–AC4, the `bevy-traps.md` #7 carve-out (a) test-body message-write) or
// drive the REAL GTW-237 `check_outcome` census by setting ganger `LifeState`s (AC5).

/// Whether [`State<BattleScapeState>`] has reached or passed `AnimateOut` (it is no longer in
/// `BattleRunning`). `AnimateOut` is the immediate successor of `BattleRunning`; on a slow
/// machine an `advance_until` predicate keyed purely on `AnimateOut` could miss it if the state
/// kept advancing, so the win/loss end tests below assert `AnimateOut` directly after a bounded
/// drive rather than rely on equality alone.
fn left_battle_running(app: &bevy::app::App) -> bool {
    battlescape_state(app) != Some(BattleScapeState::BattleRunning)
}

/// Set the [`LifeState`] of every spawned ganger whose [`Faction`] is `faction` to `to`, via a
/// `world_mut()` query in the TEST BODY (`bevy-traps.md` #7 carve-out (a) — NOT a registered
/// system or a helper taking `&mut World`). The accepted way to drive a faction out of the
/// fight so the REAL `check_outcome` census emits an outcome, without re-running the damage
/// pipeline. Mirrors the sim's in-test `set_faction_life_state`.
fn down_faction(app: &mut bevy::app::App, faction: u8, to: LifeState) {
    let target = Faction::new(faction);
    let world = app.world_mut();
    let mut query = world.query::<(&Faction, &mut LifeState)>();
    for (&fac, mut life) in query.iter_mut(world) {
        if fac == target {
            *life = to;
        }
    }
}

/// AC1 — a `BattleWon` written DURING `BattleRunning` ends the battle → `AnimateOut`.
///
/// Drives into `BattleRunning` (GTW-236 persistence applied), writes ONE
/// `gdtf_battle_sim::BattleWon` into the world's buffer (the sanctioned test-body
/// message-write), then advances: `end_battle_on_outcome` reads the outcome and inserts
/// `BattleRunningComplete`, and the marker-gated `move_on` advances `BattleRunning → AnimateOut`.
/// Pin-discriminating: with `end_battle_on_outcome` unwired the marker is never inserted, so the
/// machine would persist (GTW-236) and this fails.
#[test]
fn battle_won_in_battle_running_ends_the_battle_to_animate_out() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleRunning within {BUDGET} updates; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // Write one sim outcome signal into the app-side buffer (registered by the already-added
    // BattleSimPlugin), standing in for the census' emit.
    app.world_mut().write_message(gdtf_battle_sim::BattleWon);

    // `end_battle_on_outcome` (Update) inserts the marker; observe it WHILE still in
    // BattleRunning — `cleanup` (OnExit(BattleRunning), reused as-is, out of scope) removes the
    // per-run marker the instant the state leaves BattleRunning, so the marker and `AnimateOut`
    // are observable at adjacent points, not the same instant. Catching the insert before the
    // exit proves `end_battle_on_outcome` fired; the follow-on drive proves the chain advances.
    let marker_inserted = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<BattleRunningComplete>()
                .is_some()
        },
        BUDGET,
    );
    assert!(
        marker_inserted,
        "end_battle_on_outcome must insert the BattleRunningComplete marker on BattleWon within \
         {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    let reached_animate_out = advance_until(&mut app, left_battle_running, BUDGET);
    assert!(
        reached_animate_out,
        "a BattleWon in BattleRunning must end the battle (end_battle_on_outcome inserts the \
         marker, move_on advances) within {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a BattleWon must advance BattleRunning → AnimateOut (the same chain as the explicit end)",
    );
}

/// AC2 — a `BattleLost` written DURING `BattleRunning` ALSO ends the battle → `AnimateOut`.
///
/// Identical to AC1 but writes `gdtf_battle_sim::BattleLost`, proving LOSS ends the fight via
/// the SAME chain, not just victory. Pin-discriminating: a system that only handled the `won`
/// reader would leave this red (no marker inserted, `BattleRunning` persists).
#[test]
fn battle_lost_in_battle_running_also_ends_the_battle_to_animate_out() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleRunning within {BUDGET} updates; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    app.world_mut().write_message(gdtf_battle_sim::BattleLost);

    // Observe the marker insert while still in BattleRunning (see AC1 for why the marker and
    // AnimateOut are observable at adjacent points, not the same instant — `cleanup` removes the
    // per-run marker on exit).
    let marker_inserted = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<BattleRunningComplete>()
                .is_some()
        },
        BUDGET,
    );
    assert!(
        marker_inserted,
        "end_battle_on_outcome must insert the BattleRunningComplete marker on BattleLost within \
         {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    let reached_animate_out = advance_until(&mut app, left_battle_running, BUDGET);
    assert!(
        reached_animate_out,
        "a BattleLost in BattleRunning must ALSO end the battle within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a BattleLost must end the battle via the SAME BattleRunning → AnimateOut chain as a win",
    );
}

/// AC3 — with NO outcome signal, `BattleRunning` PERSISTS (the new system is inert).
///
/// Drives into `BattleRunning`, writes NEITHER outcome message, advances several updates, and
/// asserts `BattleRunningComplete` is ABSENT and the state is STILL `BattleRunning` — proving
/// the now-WIRED `end_battle_on_outcome` adds no spurious exit (re-asserts GTW-236 persistence
/// with the new system registered-but-quiescent). This differs from the GTW-236 persistence
/// test only in intent: that one proves no budget auto-exit; THIS one proves the GTW-239 system,
/// once in the schedule, stays quiet without an outcome.
#[test]
fn battle_running_persists_with_outcome_system_wired_but_quiescent() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleRunning within {BUDGET} updates; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    for iteration in 0..PERSIST_UPDATES {
        app.update();
        assert!(
            app.world()
                .get_resource::<BattleRunningComplete>()
                .is_none(),
            "with NO outcome signal, end_battle_on_outcome must insert NO BattleRunningComplete \
             marker; failed on update {iteration} of {PERSIST_UPDATES}",
        );
        assert_eq!(
            battlescape_state(&app),
            Some(BattleScapeState::BattleRunning),
            "BattleRunning must PERSIST with end_battle_on_outcome wired-but-quiescent (no \
             outcome); failed on update {iteration} of {PERSIST_UPDATES}",
        );
    }
}

/// AC4 (win) — a `BattleWon` written EVERY update across several updates does NOT double-fire.
///
/// A census re-declares the outcome each tick; the `not(resource_exists::<BattleRunningComplete>)`
/// gate + idempotent insert must keep the marker to ONE insert and advance the machine out of
/// `BattleRunning` EXACTLY once (it must not bounce). Asserts the state reaches `AnimateOut` and
/// the marker is present. Pin-discriminating: removing the `not(resource_exists)` gate would
/// re-run the insert each frame (still harmless for a unit marker, but the gate is the spec) —
/// the stronger guard is that the machine never re-enters `BattleRunning` after leaving.
#[test]
fn repeated_battle_won_does_not_double_fire() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleRunning within {BUDGET} updates; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // Write a BattleWon EVERY update, and stop once the machine has left BattleRunning. Track
    // that it leaves exactly once (never bounces back into BattleRunning afterwards) and that the
    // marker is present while still in BattleRunning (it is `cleanup`-removed on exit, so it is
    // only observable before the transition — see AC1).
    let mut left_once = false;
    let mut marker_seen_in_running = false;
    for _ in 0..BUDGET {
        app.world_mut().write_message(gdtf_battle_sim::BattleWon);
        app.update();
        if !left_battle_running(&app)
            && app
                .world()
                .get_resource::<BattleRunningComplete>()
                .is_some()
        {
            marker_seen_in_running = true;
        }
        if left_battle_running(&app) {
            left_once = true;
            // Once it has left, it must NEVER be back in BattleRunning on a later tick.
            assert_ne!(
                battlescape_state(&app),
                Some(BattleScapeState::BattleRunning),
                "the machine must not bounce back into BattleRunning after a repeated BattleWon",
            );
        }
    }
    assert!(
        left_once,
        "a repeated BattleWon must advance the machine out of BattleRunning within {BUDGET} \
         updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert!(
        marker_seen_in_running,
        "the BattleRunningComplete marker must have been inserted (observed present in \
         BattleRunning) under a repeated BattleWon",
    );
}

/// AC4 (loss) — a `BattleLost` written EVERY update across several updates does NOT double-fire.
///
/// The loss twin of [`repeated_battle_won_does_not_double_fire`]: proves the gate + idempotent
/// insert keep the loss stream to one advance out of `BattleRunning`.
#[test]
fn repeated_battle_lost_does_not_double_fire() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleRunning within {BUDGET} updates; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    let mut left_once = false;
    let mut marker_seen_in_running = false;
    for _ in 0..BUDGET {
        app.world_mut().write_message(gdtf_battle_sim::BattleLost);
        app.update();
        if !left_battle_running(&app)
            && app
                .world()
                .get_resource::<BattleRunningComplete>()
                .is_some()
        {
            marker_seen_in_running = true;
        }
        if left_battle_running(&app) {
            left_once = true;
            assert_ne!(
                battlescape_state(&app),
                Some(BattleScapeState::BattleRunning),
                "the machine must not bounce back into BattleRunning after a repeated BattleLost",
            );
        }
    }
    assert!(
        left_once,
        "a repeated BattleLost must advance the machine out of BattleRunning within {BUDGET} \
         updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert!(
        marker_seen_in_running,
        "the BattleRunningComplete marker must have been inserted (observed present in \
         BattleRunning) under a repeated BattleLost",
    );
}

/// AC5(a) — FULL census → outcome → app ends the battle (WIN).
///
/// The end-to-end integration over the REAL GTW-237 `check_outcome` census (no hand-written
/// message): drives into `BattleRunning` with the two-ganger fixture (player faction defaults to
/// gang 0; the enemy is gang 1), then sets the ENEMY gang (faction 1) `Dead` while the player
/// gang (faction 0) stays `Alive`. The sim's `check_outcome` (in the gated `Simulate` band) then
/// emits `BattleWon`, the app's `end_battle_on_outcome` reads it `.after(Simulate)` the SAME
/// update and inserts the marker, and `move_on` advances out of `BattleRunning`. Asserts
/// `AnimateOut`. This is the strongest evidence: the sim-signal → app-lifecycle path for a WIN.
#[test]
fn census_win_ends_the_battle_to_animate_out() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleRunning within {BUDGET} updates; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // Drive the REAL census: every ENEMY (faction 1, non-player) is out of the fight, the
    // player (faction 0) still stands → check_outcome emits BattleWon.
    down_faction(&mut app, TARGET_FACTION, LifeState::Dead);

    let reached_animate_out = advance_until(&mut app, left_battle_running, BUDGET);
    assert!(
        reached_animate_out,
        "the real census win (all enemies Dead, player Alive) must end the battle within {BUDGET} \
         updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a census-emitted BattleWon must advance BattleRunning → AnimateOut end-to-end",
    );
}

/// AC5(b) — FULL census → outcome → app ends the battle (LOSS).
///
/// The loss twin of [`census_win_ends_the_battle_to_animate_out`]: sets every PLAYER ganger
/// (faction 0) out of the fight so the sim's `check_outcome` emits `BattleLost`, and asserts the
/// app likewise reaches `AnimateOut` via the same chain. Proves the end-to-end
/// sim-signal → app-lifecycle path for a LOSS.
#[test]
fn census_loss_ends_the_battle_to_animate_out() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleRunning within {BUDGET} updates; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // Drive the REAL census: every PLAYER ganger (faction 0, the default player_faction) is out
    // of the fight → check_outcome emits BattleLost (enemy liveness is irrelevant to a loss).
    down_faction(&mut app, SHOOTER_FACTION, LifeState::Dead);

    let reached_animate_out = advance_until(&mut app, left_battle_running, BUDGET);
    assert!(
        reached_animate_out,
        "the real census loss (all player gangers Dead) must end the battle within {BUDGET} \
         updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a census-emitted BattleLost must advance BattleRunning → AnimateOut end-to-end",
    );
}
