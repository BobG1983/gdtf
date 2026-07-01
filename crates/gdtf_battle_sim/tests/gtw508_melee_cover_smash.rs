//! GTW-508 (child GTW-37d of GTW-37) — the LIVE melee-vs-STRUCTURE act: a TU-costed close-combat
//! strike against an adjacent inert Cover / Wall cell, resolved as an UNCONTESTED cover-smash
//! (NO opposed Fight roll, NO `FightRng` draw) that applies multiplied weapon damage to the
//! structure's HP through the EXISTING cover ledger. Proven END-TO-END on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path, driven THROUGH a buffered
//! `MeleeRequested::new_structural` (the same message + constructor the input seam writes).
//!
//! The clause contract this covers:
//!
//! - **C5(a) — a smash reduces cover HP**: a melee strike on an adjacent Cover cell REDUCES that
//!   cell's structure HP in the real `CoverLedger`, spends the attacker's TU, and emits a
//!   `MeleeResolved` (the strike-glyph FX signal). PIN-DISCRIMINATING (fails if the structural
//!   arm is unwired).
//! - **C5(b) — repeated/sufficient smashing destroys it + the cover-destroyed signal fires**:
//!   enough melee strikes deplete the cell to zero, at which point the EXISTING `CoverDestroyed`
//!   signal is emitted (the presenter's GTW-386 rubble-burst FX reacts to it verbatim).
//! - **C5(c) — the structural path takes NO `FightRng` draw**: the outcome is IDENTICAL under two
//!   DIFFERENT battle seeds (an inert structure is not rolled against — no opposed Fight, no
//!   `FightRng`/`ShotRng`/`SeverityRng` draw), so the cover-smash is seed-independent.
//!
//! NO pinned tunable magnitudes: the tests assert HP-DECREASED / destroyed-on-lethal /
//! seed-independence — never a specific `mult_max` value or a specific weapon-damage number.
//!
//! HARNESS NOTE (the gtw507 idiom): the sim crate is the LOW crate, so it cannot dev-dep
//! `gdtf_test_utils` (a cycle). The established sim-crate battle-integration idiom drives
//! `setup_battle_on_request` via a `SetupBattleRequested` message against a `MinimalPlugins` +
//! `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT production wiring.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins, Resource},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Cool, Faction, Grit, Speed, Stance, StanceKind, Strength, Toughness, Tu,
    acts::{MeleeRequested, MeleeResolved},
    armor::{ArmorHardness, ArmorProtection},
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, Facing, GangRegistry},
    metric::{Cell, CellLevel, Level},
    occupancy_sync::CoverDestroyed,
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
};

/// Gang `0` is the player attacker.
const PLAYER: u8 = 0;

/// A view range comfortably covering an adjacent smash (arbitrary test tuning).
const TEST_VIEW_RANGE: u16 = 12;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// Build the FULL live-runtime harness (the gtw507 `battle_app` idiom) with `seed` for the RNG
/// streams and a `CombatTuning` carrying the view range + the DEFAULT melee tuning (its
/// `mult_max` is the FORK-4a structural multiplier), plus the persistent `Load` weapon/armor
/// registries a `MinimalPlugins` app has no `AssetServer` to load.
fn battle_app(seed: u64) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    // The melee registry (with the `fists` default) so the attacker's melee weapon resolves at
    // setup (a fixture ganger authors none → `fists`, a damage-9 / punch-3 melee spec).
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    (app, seed)
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path with `seed` and settle it
/// (the deferred `bsn!` ganger scenes materialize and the first-run recompute fills the fog).
fn drive_setup(app: &mut App, seed: u64, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(seed)));
    for _ in 0..4 {
        app.update();
    }
}

/// The entity of the (sole) player ganger.
fn player_ganger(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == PLAYER)
        .map(|(entity, _)| entity)
}

/// The current TU of `entity`.
fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

/// The current HP of the cover at `at` in the model ledger, or `None` if the cell has no entry.
fn cover_hp(app: &App, at: CellLevel) -> Option<u32> {
    app.world()
        .get_resource::<CoverLedger>()
        .and_then(|ledger| ledger.peek(&at))
        .map(|entry| *entry.current_hp)
}

/// Whether the cover at `at` is marked destroyed in the model ledger.
fn cover_destroyed_flag(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<CoverLedger>()
        .and_then(|ledger| ledger.peek(&at))
        .is_some_and(|entry| *entry.destroyed)
}

// ── Test-local recorders (a MessageReader only sees the current+previous update). ──

/// Every `MeleeResolved` observed across the run.
#[derive(Resource, Default)]
struct MeleeLog {
    /// One entry per `MeleeResolved` emitted.
    hits: Vec<MeleeResolved>,
}

/// Every `CoverDestroyed` observed across the run.
#[derive(Resource, Default)]
struct DestroyedLog {
    /// One entry per `CoverDestroyed` emitted.
    hits: Vec<CoverDestroyed>,
}

/// Drain `MeleeResolved` into the recorder.
fn record_melee(
    mut resolved: bevy::prelude::MessageReader<MeleeResolved>,
    mut log: bevy::prelude::ResMut<MeleeLog>,
) {
    for hit in resolved.read() {
        log.hits.push(*hit);
    }
}

/// Drain `CoverDestroyed` into the recorder.
fn record_destroyed(
    mut destroyed: bevy::prelude::MessageReader<CoverDestroyed>,
    mut log: bevy::prelude::ResMut<DestroyedLog>,
) {
    for hit in destroyed.read() {
        log.hits.push(*hit);
    }
}

/// Add both recorders (after `BattleSimPlugin`, so the buffers exist).
fn with_logs(app: &mut App) {
    app.init_resource::<MeleeLog>();
    app.init_resource::<DestroyedLog>();
    app.add_systems(bevy::app::Update, (record_melee, record_destroyed));
}

/// How many `MeleeResolved` were emitted across the run.
fn melee_hits(app: &App) -> usize {
    app.world()
        .get_resource::<MeleeLog>()
        .map_or(0, |log| log.hits.len())
}

/// How many `CoverDestroyed` were emitted across the run.
fn destroyed_hits(app: &App) -> usize {
    app.world()
        .get_resource::<DestroyedLog>()
        .map_or(0, |log| log.hits.len())
}

/// An attacker with a strong Fight — irrelevant to the structural path (no opposed roll), but a
/// full-stat fielded ganger so the melee weapon + TU pool resolve normally.
fn attacker(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(12.0))
        .build()
}

/// Seed an intact ARMORLESS cover piece at `at` in the model ledger with `max_hp` HP — the
/// structure the smash tests hit. Armorless (protection / hardness 0) so a connecting smash
/// removes HP (the tests assert HP moved, never a pinned magnitude). The `CoverLedger` is the
/// model surface the smash spends; seeding it directly is the gtw507 test idiom.
fn seed_cover(app: &mut App, at: CellLevel, max_hp: u32) {
    let entry = CoverEntry::seeded(
        CoverHp::new(max_hp),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
    );
    app.world_mut()
        .resource_mut::<CoverLedger>()
        .insert(at, entry);
}

/// Step `app` a fixed number of ticks so a written `MeleeRequested` dispatches + the recorders
/// capture any emitted signals.
fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

// === C5(a) — a smash on an adjacent Cover cell REDUCES its HP, spends TU, emits MeleeResolved. ===

#[test]
fn smash_reduces_adjacent_cover_hp_and_emits_resolved() {
    let (mut app, seed) = battle_app(0x5508_0A0A);
    with_logs(&mut app);

    // The player attacker faces East at (5,5); the cover cell is directly east at (6,5), 8-adjacent.
    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    // A durable adjacent wall — big enough that ONE smash damages but does not destroy it (so this
    // isolates the "HP reduced" clause from the "destroyed" clause).
    let cover = ground(6, 5);
    seed_cover(&mut app, cover, 1_000);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };
    let (Some(tu_before), Some(hp_before)) = (tu_of(&app, attacker_entity), cover_hp(&app, cover))
    else {
        unreachable!("the attacker carries Tu and the cover cell was seeded");
    };

    // Drive the smash THROUGH the buffered MeleeRequested::new_structural (the structural form
    // the input seam writes).
    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker_entity, cover));
    step(&mut app, 3);

    // C5(a): the cover's HP went DOWN through the REAL ledger.
    let Some(hp_after) = cover_hp(&app, cover) else {
        unreachable!("the cover entry persists");
    };
    assert!(
        hp_after < hp_before,
        "C5(a): a melee cover-smash REDUCES the cell's structure HP ({hp_after} < {hp_before})",
    );

    // C5(a): the attacker's TU was spent (a swing at a structure costs TU like a swing at a ganger).
    assert!(
        tu_of(&app, attacker_entity).is_some_and(|tu| tu < tu_before),
        "C5(a): the attacker's TU is spent by the smash",
    );

    // C5(a): a MeleeResolved (the strike-glyph FX signal) was emitted — the structural act
    // resolved end-to-end on the real runtime path. PIN-DISCRIMINATING (fails if unwired).
    assert!(
        melee_hits(&app) >= 1,
        "C5(a): a melee cover-smash emits MeleeResolved (the FX signal) — wired end-to-end",
    );
}

// === C5(b) — sufficient/repeated smashing DESTROYS the cover and fires the cover-destroyed signal. ===

#[test]
fn repeated_smashing_destroys_cover_and_fires_the_destroyed_signal() {
    let (mut app, seed) = battle_app(0x5508_0B0B);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    // A FRAGILE adjacent wall (1 HP, armorless) so a single connecting smash depletes it to zero
    // — no pinned magnitude, just "at least one smash's damage ≥ 1 HP" (the armorless fists deal
    // real damage). Structuring it this way keeps the destroy clause magnitude-free.
    let cover = ground(6, 5);
    seed_cover(&mut app, cover, 1);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };

    // Smash repeatedly — each swing re-tops the attacker's TU is NOT modelled here, but the
    // structural depletion is CUMULATIVE across strikes (the ledger persists current_hp), and a
    // fragile 1-HP wall is destroyed on the first connecting hit. Drive a few to be robust to the
    // TU pool (a real fielded attacker has ample TU for several fists swings).
    for _ in 0..3 {
        app.world_mut()
            .write_message(MeleeRequested::new_structural(attacker_entity, cover));
        step(&mut app, 3);
    }

    // C5(b): the cover was destroyed (HP depleted to zero) — read the model ledger's destroyed flag.
    assert!(
        cover_destroyed_flag(&app, cover),
        "C5(b): sufficient melee smashing DESTROYS the cover (its ledger HP reached zero)",
    );
    assert_eq!(
        cover_hp(&app, cover),
        Some(0),
        "C5(b): a destroyed cover's current HP is zero",
    );

    // C5(b): the EXISTING CoverDestroyed signal fired (the presenter's GTW-386 FX reacts to it).
    // PIN-DISCRIMINATING: with the cover-destroyed bridge unwired this would be zero.
    assert!(
        destroyed_hits(&app) >= 1,
        "C5(b): destroying the cover fires the CoverDestroyed signal (the GTW-386 FX bridge)",
    );
    // And a MeleeResolved was emitted for the smash too (the strike-glyph FX).
    assert!(
        melee_hits(&app) >= 1,
        "C5(b): the smash emits MeleeResolved",
    );
}

// === C5(c) — the structural path takes NO FightRng draw: the outcome is SEED-INDEPENDENT. ===

#[test]
fn the_structural_smash_is_seed_independent_no_fight_roll() {
    // Resolve the same smash on the same durable wall under TWO DIFFERENT battle seeds and read
    // the resulting cover HP. A contested (opposed-Fight) path would draw from FightRng and vary
    // with the seed; the UNCONTESTED structural path takes NO RNG draw, so the HP-after is
    // IDENTICAL across seeds — the C5(c) "no FightRng draw" property.
    let run_with_seed = |seed: u64| -> Option<u32> {
        let (mut app, seed) = battle_app(seed);
        with_logs(&mut app);
        let situation = SituationBuilder::new()
            .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
            .build_with_gangs();
        drive_setup(&mut app, seed, situation);
        let cover = ground(6, 5);
        seed_cover(&mut app, cover, 1_000);
        let Some(attacker_entity) = player_ganger(&mut app) else {
            unreachable!("setup spawns one player attacker");
        };
        app.world_mut()
            .write_message(MeleeRequested::new_structural(attacker_entity, cover));
        step(&mut app, 3);
        cover_hp(&app, cover)
    };

    let a = run_with_seed(0x0000_1111);
    let b = run_with_seed(0xFFFF_EEEE);
    assert_eq!(
        a, b,
        "C5(c): the structural smash is SEED-INDEPENDENT (no opposed roll, no FightRng draw) — \
         two different seeds leave the same cover HP: {a:?} vs {b:?}",
    );
    // And it's a non-trivial outcome (the smash actually removed HP under both seeds — else the
    // seed-independence is vacuous).
    assert!(
        a.is_some_and(|hp| hp < 1_000),
        "precondition: the smash actually reduced the cover HP (a real, non-vacuous outcome)",
    );
}

// === A gate check: a NON-ADJACENT structure cell produces no smash (the 8-adjacency gate). ===

#[test]
fn a_non_adjacent_structure_is_not_smashed() {
    let (mut app, seed) = battle_app(0x5508_0C0C);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    // A wall THREE cells east — well outside the 8-adjacent reach.
    let far_cover = ground(8, 5);
    seed_cover(&mut app, far_cover, 1_000);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };
    let Some(hp_before) = cover_hp(&app, far_cover) else {
        unreachable!("the far cover cell was seeded");
    };

    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker_entity, far_cover));
    step(&mut app, 3);

    assert_eq!(
        cover_hp(&app, far_cover),
        Some(hp_before),
        "the 8-adjacency gate held — a non-adjacent structure takes NO melee damage",
    );
    assert_eq!(
        melee_hits(&app),
        0,
        "a non-adjacent structure smash emits NO MeleeResolved",
    );
}
