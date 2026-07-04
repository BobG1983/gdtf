//! GTW-507 (child GTW-37c of GTW-37) — the LIVE melee ACT: a TU-costed close-combat strike
//! from 8-adjacency with clear LOS against an alive opposing ganger, resolved through the §7
//! opposed-Fight → §5 damage → §6 wound synthesis. Proven END-TO-END on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path, driven THROUGH a
//! buffered `MeleeRequested` (the same message the input seam writes).
//!
//! The acceptance criteria (C6 a / b / c / e):
//!
//! - **a — connect applies damage** (forced): an attacker 8-adjacent + in-LOS of an alive
//!   opposing ganger, on a FORCED connect (variance 0 + a zero-Fight defender → the degenerate
//!   `def ≤ 0` connect at `mult_max`), takes the target's HP DOWN, records a Wound, spends the
//!   attacker's TU, AND emits a `MeleeResolved`. PIN-DISCRIMINATING (fails if any link unwired).
//! - **b — miss applies nothing**: with the opposed roll LOST (variance 0 + a zero-Fight
//!   ATTACKER vs a Fight-positive defender → `atk ≤ def`, no connect), the target loses NO HP,
//!   records NO Wound, and NO `MeleeResolved` is emitted (a clean miss) — yet the attacker's TU
//!   IS spent (a swing costs TU whether or not it lands).
//! - **c — the gates** (each a discriminating case): a NON-ADJACENT target, a LOS-BLOCKED
//!   target, a SAME-FACTION (ally) target, and a DEAD target each produce NO melee (no HP loss,
//!   no `MeleeResolved`).
//! - **e — in-engine QA evidence (headless)**: the connect case proves the act resolves AND the
//!   `MeleeResolved` FX signal emits end-to-end on the real runtime path.
//!
//! DETERMINISM: a seeded battle RNG + a degenerate Fight on one side, so the outcome is
//! variance-INDEPENDENT (no `variance 0` — `opposed_fight` draws `random_range(1−v..1+v)`, which
//! is an empty range at `v == 0`). A ZERO-Fight DEFENDER drives the §7 degenerate `def ≤ 0`
//! connect (guaranteed connect at `mult_max`, any variance); a ZERO-Fight ATTACKER drives a
//! guaranteed MISS (`atk == 0 ≤ def > 0`, any variance) — the connect / miss outcome is a pure
//! function of the two gangers' Fights, with no brittle tunable-magnitude assert.
//!
//! HARNESS NOTE (the gtw468 / gtw355 idiom): the sim crate is the LOW crate, so it cannot dev-dep
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
    ArmorBroken, ArmorIntegrity, Cool, Faction, Grit, Hp, LifeState, Position, Speed, Stance,
    StanceKind, Strength, Toughness, Tu, Wears, Wounds,
    acts::{MeleeRequested, MeleeResolved},
    armor::{ArmorHardness, ArmorProtection},
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, Facing, GangRegistry},
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
};

/// An arbitrary (not shipped tuning) seed for the test battle's RNG streams.
const SEED: u64 = 0x5E1E_9907;

/// Gang `0` is the player; gang `1` is the enemy.
const PLAYER: u8 = 0;
const ENEMY: u8 = 1;

/// A view range comfortably covering an 8-adjacent strike (arbitrary test tuning).
const TEST_VIEW_RANGE: u16 = 12;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// Build the FULL live-runtime harness (the gtw468 `battle_app` idiom) with a `CombatTuning`
/// carrying the view range + the DEFAULT melee tuning (its `variance > 0`, so `opposed_fight`'s
/// `random_range(1−v..1+v)` draw is a valid non-empty range), plus the persistent `Load`
/// weapon/armor registries a `MinimalPlugins` app has no `AssetServer` to load.
fn battle_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee weapon
    // resolves at setup (fixture gangers author none → `fists`, which the test registry maps to
    // a damage-9 / punch-3 melee spec — guaranteeing a connecting hit penetrates the test armor).
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path and settle it (the deferred
/// `bsn!` ganger scenes materialize and the first-run recompute fills the fog).
fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    for _ in 0..4 {
        app.update();
    }
}

/// The entity of the (sole) ganger of `faction`.
fn ganger_of(app: &mut App, faction: u8) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
}

/// The current TU of `entity`.
fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

/// The current HP of `entity`.
fn hp_of(app: &App, entity: Entity) -> Option<u16> {
    app.world().get::<Hp>(entity).map(|h| **h)
}

/// The current Wounds of `entity`.
fn wounds_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Wounds>(entity).map(|w| **w)
}

/// The current `LifeState` of `entity`.
fn life_of(app: &App, entity: Entity) -> Option<LifeState> {
    app.world().get::<LifeState>(entity).copied()
}

// ── A test-local MeleeResolved recorder ─────────────────────────────────────────
//
// `dispatch_melee` emits a MeleeResolved per connecting hit; a `MessageReader` only drains a
// buffer once and only sees the current+previous update. To assert across the whole run we
// record every MeleeResolved into a resource the moment it is emitted.

/// Every `MeleeResolved` observed across the run — a test-only recorder so the assertion reads
/// the full history rather than racing the one-update message lifetime.
#[derive(Resource, Default)]
struct MeleeLog {
    /// One entry per `MeleeResolved` emitted (the struck cell + damage type carried).
    hits: Vec<MeleeResolved>,
}

/// Drain `MeleeResolved` into the recorder — added to the test app so the run's full strike
/// history is queryable after `app.update()`s.
fn record_melee(
    mut resolved: bevy::prelude::MessageReader<MeleeResolved>,
    mut log: bevy::prelude::ResMut<MeleeLog>,
) {
    for hit in resolved.read() {
        log.hits.push(*hit);
    }
}

/// Add the `MeleeResolved` + `MeleeStruck` + `ArmorBroken` recorders (after
/// `BattleSimPlugin`, so the buffers exist).
fn with_melee_log(app: &mut App) {
    app.init_resource::<MeleeLog>();
    app.init_resource::<StruckLog>();
    app.init_resource::<BrokenLog>();
    app.add_systems(
        bevy::app::Update,
        (record_melee, record_struck, record_broken),
    );
}

/// How many `MeleeResolved` were emitted across the run.
fn melee_hits(app: &App) -> usize {
    app.world()
        .get_resource::<MeleeLog>()
        .map_or(0, |log| log.hits.len())
}

/// Every `MeleeStruck` observed across the run — the GTW-572 number-bearing melee fact the
/// combat log's melee-damage line reads (attacker + target + applied HP loss).
#[derive(Resource, Default)]
struct StruckLog {
    /// One entry per `MeleeStruck` emitted.
    facts: Vec<gdtf_battle_sim::MeleeStruck>,
}

/// Drain `MeleeStruck` into the recorder.
fn record_struck(
    mut struck: bevy::prelude::MessageReader<gdtf_battle_sim::MeleeStruck>,
    mut log: bevy::prelude::ResMut<StruckLog>,
) {
    for fact in struck.read() {
        log.facts.push(*fact);
    }
}

/// The recorded `MeleeStruck` facts across the run.
fn struck_facts(app: &App) -> Vec<gdtf_battle_sim::MeleeStruck> {
    app.world()
        .get_resource::<StruckLog>()
        .map_or_else(Vec::new, |log| log.facts.clone())
}

/// Every `ArmorBroken` observed across the run — the GTW-572 protecting→broken fact the
/// armor-broken pop + combat-log line drain. Recorded here so the connect test pins the
/// REAL melee emission path (the verb's surfaced `MeleeStrike.wear` → the dispatch
/// bridge → the buffered message), not a hand-written buffer write.
#[derive(Resource, Default)]
struct BrokenLog {
    /// One entry per `ArmorBroken` emitted.
    facts: Vec<ArmorBroken>,
}

/// Drain `ArmorBroken` into the recorder.
fn record_broken(
    mut broken: bevy::prelude::MessageReader<ArmorBroken>,
    mut log: bevy::prelude::ResMut<BrokenLog>,
) {
    for fact in broken.read() {
        log.facts.push(*fact);
    }
}

/// The recorded `ArmorBroken` facts across the run.
fn broken_facts(app: &App) -> Vec<ArmorBroken> {
    app.world()
        .get_resource::<BrokenLog>()
        .map_or_else(Vec::new, |log| log.facts.clone())
}

/// Reduce every worn piece on `ganger` to a NEAR-BROKEN integrity (1), returning how
/// many pieces were reduced (the caller asserts the fixture actually wears armor).
///
/// The §5 formula never reads integrity magnitude (only the `> 0` protects gate), so
/// this leaves the connect test's damage / wound / TU outcomes byte-identical — it only
/// guarantees the connecting strike's positive wear (`min(protection, damage) ≥ 2` for
/// the test armor) CROSSES the struck piece protecting→broken.
fn wear_pieces_near_broken(app: &mut App, ganger: Entity) -> usize {
    let pieces: Vec<Entity> = app
        .world()
        .get::<Wears>(ganger)
        .map(|wears| wears.pieces().collect())
        .unwrap_or_default();
    for &piece in &pieces {
        if let Some(mut integrity) = app.world_mut().get_mut::<ArmorIntegrity>(piece) {
            *integrity = ArmorIntegrity::new(1);
        }
    }
    pieces.len()
}

/// An attacker ganger with a strong Fight (high Strength / Speed / Grit / Cool) so its rolled
/// Fight clearly beats a zero-Fight defender's (the forced-connect case under variance 0).
fn strong_attacker(
    at: CellLevel,
    faction: u8,
    facing: Direction,
) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .build()
}

/// A defenceless-in-melee target: ZERO Fight contributors (Strength / Speed / Grit / Cool all 0)
/// so a forced strike connects (the §7 degenerate `def ≤ 0` path), with a MODERATE Toughness
/// (`Hp = grit·Grit + toughness·Toughness + cool·Cool`) — enough HP that the strike's outcome is
/// observable, but low enough that the multiplied §6 penetrating damage clears the severity
/// floor so a WOUND (or a lethal down) registers, not a fully-mitigated graze.
fn defenceless_target(at: CellLevel, faction: u8) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(0.0))
        .strength(Strength::new(0.0))
        .grit(Grit::new(0.0))
        .cool(Cool::new(0.0))
        // Moderate Toughness: a real HP pool to lose, but not so much that the §6 score's
        // `−k·Toughness` mitigation pushes every connecting hit down to a graze.
        .toughness(Toughness::new(12.0))
        .build()
}

/// A Fight-positive target (some Strength / Speed) so a ZERO-Fight attacker's strike does NOT
/// connect under variance 0 (`atk == 0 ≤ def > 0`) — the miss case's defender.
fn fighting_target(at: CellLevel, faction: u8) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(120.0))
        .build()
}

/// A ZERO-Fight attacker (every Fight contributor 0) so under variance 0 its rolled Fight is 0
/// and a strike against any Fight-positive defender is a MISS (`atk ≤ def`).
fn weak_attacker(
    at: CellLevel,
    faction: u8,
    facing: Direction,
) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(0.0))
        .strength(Strength::new(0.0))
        .grit(Grit::new(0.0))
        .cool(Cool::new(0.0))
        .build()
}

/// Step `app` a fixed number of ticks so a written `MeleeRequested` dispatches + the recorder
/// captures any `MeleeResolved`.
fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

// === C6(a) + C6(e) — a forced connect applies damage end-to-end: HP down + Wound + TU spent +
// MeleeResolved emitted (the FX signal proves the act resolves end-to-end on the real path). ===

#[test]
fn connect_applies_damage_and_emits_resolved() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    // The player attacker faces East at (5,5); the defenceless enemy stands 8-adjacent at (6,5)
    // (directly east, clear LOS, no cover between them).
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player attacker and one enemy target");
    };
    let (Some(attacker_tu_before), Some(target_hp_before), Some(target_wounds_before)) = (
        tu_of(&app, attacker),
        hp_of(&app, target),
        wounds_of(&app, target),
    ) else {
        unreachable!("both gangers carry Tu / Hp / Wounds pools");
    };

    // GTW-572: reduce the target's worn test armor to NEAR-BROKEN (integrity 1) so the
    // forced connect's positive wear crosses it protecting→broken — pinning the melee
    // ArmorBroken emission on the real path (verb wear verdict → dispatch bridge →
    // buffered fact). Integrity magnitude never feeds §5, so every other assert below
    // is untouched.
    let reduced = wear_pieces_near_broken(&mut app, target);
    assert!(
        reduced > 0,
        "fixture precondition: the target wears the test armor (pieces to reduce)",
    );

    // Drive the strike THROUGH the buffered MeleeRequested (the message the input seam writes).
    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    // C6(a): the target's HP went DOWN (a connecting hit applies the multiplied §5/§6 damage).
    let Some(target_hp_after) = hp_of(&app, target) else {
        unreachable!("the target persists");
    };
    assert!(
        target_hp_after < target_hp_before,
        "C6(a): a connecting melee hit takes the target's HP DOWN ({target_hp_after} < \
         {target_hp_before})",
    );

    // C6(a): a Wound was recorded (the §6 severity tier spent from the Wounds pool). The forced
    // connect at mult_max + the defenceless target makes a non-graze wound the expected outcome;
    // structurally, Wounds did not RISE and the pool moved (a wound was registered).
    let Some(target_wounds_after) = wounds_of(&app, target) else {
        unreachable!("the target persists");
    };
    assert!(
        target_wounds_after <= target_wounds_before,
        "C6(a): the connecting hit never RAISES the target's Wounds",
    );
    // The fat-HP defenceless target survives but takes a real wound: assert a Wound was spent
    // OR (if the multiplied blow was lethal) the target is Dead — either way the §6 step ran.
    let target_downed_or_dead = matches!(
        life_of(&app, target),
        Some(LifeState::Downed | LifeState::Dead)
    );
    assert!(
        target_wounds_after < target_wounds_before || target_downed_or_dead,
        "C6(a): the §6 wound step ran — a Wound was spent (or the blow was lethal): \
         {target_wounds_before} → {target_wounds_after}",
    );

    // C6(a): the attacker's TU was spent (the fight-mode flat TU charge).
    let Some(attacker_tu_after) = tu_of(&app, attacker) else {
        unreachable!("the attacker persists");
    };
    assert!(
        attacker_tu_after < attacker_tu_before,
        "C6(a): the attacker's TU is spent by the strike ({attacker_tu_after} < \
         {attacker_tu_before})",
    );

    // C6(a) + C6(e): a MeleeResolved was emitted (the strike-landed FX signal) — the act
    // resolved end-to-end on the real runtime path. PIN-DISCRIMINATING (fails if unwired).
    assert!(
        melee_hits(&app) >= 1,
        "C6(a)/C6(e): a connecting strike emits MeleeResolved (the FX signal) — the live melee \
         act is wired end-to-end",
    );

    // GTW-572: the connecting strike ALSO emits the number-bearing MeleeStruck fact — both
    // combatants named, carrying the strike's RESOLVED HP damage (the sim emits the FACT the
    // combat log's melee-damage line phrases). The resolved number is at LEAST the observed
    // pool delta (apply_hit saturates the pool at 0, so an overkill blow drains fewer HP than
    // it resolved — the fact carries the blow, the pool carries the floor).
    let facts = struck_facts(&app);
    let observed_loss = i32::from(target_hp_before) - i32::from(target_hp_after);
    assert!(
        facts.iter().any(|fact| fact.attacker == attacker
            && fact.target == target
            && *fact.hp_damage >= observed_loss
            && *fact.hp_damage > 0),
        "GTW-572: a connecting strike emits one MeleeStruck {{ attacker, target, hp_damage }} \
         whose resolved amount is positive and at least the observed HP delta \
         ({observed_loss}), got {facts:?}",
    );

    // GTW-572: the connecting strike on the NEAR-BROKEN worn piece crossed it
    // protecting→broken, and the dispatch bridge surfaced the verb's wear verdict as
    // EXACTLY ONE buffered ArmorBroken naming the struck ganger — the melee emission
    // pin (reverting the resolve.rs bridge or the MeleeStrike.wear surfacing fails it).
    let breaks = broken_facts(&app);
    assert_eq!(
        breaks.len(),
        1,
        "GTW-572: a connecting strike that crosses a near-broken worn piece emits exactly \
         one ArmorBroken, got {breaks:?}",
    );
    assert!(
        breaks.first().is_some_and(|broke| broke.ganger == target),
        "GTW-572: the ArmorBroken names the struck melee target, got {breaks:?}",
    );
}

// === C6(b) — a miss (the opposed roll lost) applies NO damage, records no Wound, emits no
// MeleeResolved — yet the attacker's TU IS spent (a swing costs TU regardless). ===

#[test]
fn miss_applies_no_damage_but_spends_tu() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    // A ZERO-Fight player attacker faces East at (5,5); a Fight-positive enemy stands 8-adjacent
    // at (6,5). Under variance 0, atk(0) ≤ def(>0) → the opposed roll is LOST (a miss).
    let situation = SituationBuilder::new()
        .with_gangers([
            weak_attacker(ground(5, 5), PLAYER, Direction::East),
            fighting_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player attacker and one enemy target");
    };
    let (Some(attacker_tu_before), Some(target_hp_before), Some(target_wounds_before)) = (
        tu_of(&app, attacker),
        hp_of(&app, target),
        wounds_of(&app, target),
    ) else {
        unreachable!("both gangers carry Tu / Hp / Wounds pools");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    // C6(b): NO damage — HP unchanged, Wounds unchanged.
    assert_eq!(
        hp_of(&app, target),
        Some(target_hp_before),
        "C6(b): a missed strike (opposed roll lost) takes NO HP",
    );
    assert_eq!(
        wounds_of(&app, target),
        Some(target_wounds_before),
        "C6(b): a missed strike records NO Wound",
    );
    // C6(b): NO MeleeResolved (the §7 connect gate held — a miss emits nothing).
    assert_eq!(
        melee_hits(&app),
        0,
        "C6(b): a missed strike emits NO MeleeResolved",
    );
    // C6(b): the swing STILL cost TU (a swing costs TU whether or not it connects).
    assert!(
        tu_of(&app, attacker).is_some_and(|tu| tu < attacker_tu_before),
        "C6(b): the attacker's TU is spent even on a miss (a swing costs TU regardless)",
    );
}

// === C6(c) — the GATES. Each discriminating case (non-adjacent / LOS-blocked / non-enemy /
// dead) produces NO melee: no HP loss, no MeleeResolved. ===

/// C6(c) gate 1 — a NON-ADJACENT target (Chebyshev > 1) produces no melee.
#[test]
fn gate_non_adjacent_produces_no_melee() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    // The enemy stands THREE cells east — well outside the 8-adjacent reach.
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(8, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    let Some(hp_before) = hp_of(&app, target) else {
        unreachable!("the target has Hp");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    assert_eq!(
        hp_of(&app, target),
        Some(hp_before),
        "C6(c): a non-adjacent target takes NO melee damage (the 8-adjacency gate held)",
    );
    assert_eq!(
        melee_hits(&app),
        0,
        "C6(c): a non-adjacent strike emits NO MeleeResolved",
    );
}

/// C6(c) gate 2 — a SAME-FACTION (ally) target produces no melee (no friendly melee).
#[test]
fn gate_same_faction_produces_no_melee() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    // BOTH gangers are PLAYER faction — an ally is never a melee target.
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), PLAYER),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let world = app.world_mut();
    let mut q = world.query::<(Entity, &Position)>();
    let mut entities: Vec<(Entity, CellLevel)> = q.iter(world).map(|(e, p)| (e, **p)).collect();
    entities.sort_by_key(|(_, p)| (p.z, p.y, p.x));
    let (Some(&(attacker, _)), Some(&(target, _))) = (entities.first(), entities.get(1)) else {
        unreachable!("setup spawns two player gangers");
    };
    let Some(hp_before) = hp_of(&app, target) else {
        unreachable!("the target has Hp");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    assert_eq!(
        hp_of(&app, target),
        Some(hp_before),
        "C6(c): a same-faction ally takes NO melee damage (the opposing-faction gate held)",
    );
    assert_eq!(
        melee_hits(&app),
        0,
        "C6(c): a same-faction strike emits NO MeleeResolved",
    );
}

/// C6(c) gate 3 — a DEAD target produces no melee (only an alive opposing ganger is a target).
#[test]
fn gate_dead_target_produces_no_melee() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    // Force the target DEAD (a corpse) before the strike — only an alive target is meleeable.
    app.world_mut().entity_mut(target).insert(LifeState::Dead);
    app.update();
    let Some(hp_before) = hp_of(&app, target) else {
        unreachable!("the target persists");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    assert_eq!(
        hp_of(&app, target),
        Some(hp_before),
        "C6(c): a DEAD target takes NO melee damage (the alive gate held)",
    );
    assert_eq!(
        melee_hits(&app),
        0,
        "C6(c): a strike on a dead target emits NO MeleeResolved",
    );
}

/// C6(c) gate 4 — a LOS-BLOCKED target produces no melee. The attacker and target are
/// DIAGONALLY 8-adjacent (Chebyshev 1), and BOTH orthogonal corner cells the diagonal sight ray
/// can cross are filled with a HIGH wall — so whichever corner the voxel-DDA steps through, the
/// center-to-center sight march stops on a wall BEFORE the target. Adjacency + faction + alive
/// all hold; only the LOS gate rejects the strike (so this is discriminating: with the walls
/// REMOVED the same geometry connects — proven by the other connect tests at the same range).
#[test]
fn gate_los_blocked_produces_no_melee() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    // Attacker at (5,5); target DIAGONALLY 8-adjacent at (6,6). The diagonal sight ray crosses
    // one of the two corner cells (6,5) / (5,6) — wall BOTH so it is blocked either way.
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 6), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    let Some(hp_before) = hp_of(&app, target) else {
        unreachable!("the target has Hp");
    };

    // Seed HIGH walls in BOTH corner cells the diagonal sight ray can cross, so the march stops
    // on a wall before the target regardless of the DDA's axis-step tie-break. The cover ledger
    // is the model surface `has_los` marches; seeding it directly is the test idiom.
    {
        let mut ledger = app.world_mut().resource_mut::<CoverLedger>();
        for corner in [ground(6, 5), ground(5, 6)] {
            let wall = CoverEntry::seeded(
                CoverHp::new(100),
                HeightBand::High,
                ArmorProtection::new(50),
                ArmorHardness::new(50),
            );
            ledger.insert(corner, wall);
        }
    }
    app.update();

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    assert_eq!(
        hp_of(&app, target),
        Some(hp_before),
        "C6(c): a LOS-blocked target takes NO melee damage (the LOS gate held)",
    );
    assert_eq!(
        melee_hits(&app),
        0,
        "C6(c): a LOS-blocked strike emits NO MeleeResolved",
    );
}

// === A defensive determinism check: the same seed + tuning reproduces the same connect outcome
// (HP after) through the live path. ===

#[test]
fn the_strike_outcome_is_deterministic_across_identical_runs() {
    let run_once = || {
        let mut app = battle_app();
        with_melee_log(&mut app);
        let situation = SituationBuilder::new()
            .with_gangers([
                strong_attacker(ground(5, 5), PLAYER, Direction::East),
                defenceless_target(ground(6, 5), ENEMY),
            ])
            .build_with_gangs();
        drive_setup(&mut app, situation);
        let (Some(attacker), Some(target)) =
            (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
        else {
            unreachable!("setup spawns one player and one enemy");
        };
        app.world_mut()
            .write_message(MeleeRequested::new(attacker, target));
        step(&mut app, 3);
        (hp_of(&app, target), melee_hits(&app))
    };

    let first = run_once();
    let second = run_once();
    assert_eq!(
        first, second,
        "the same seed + tuning reproduces the same strike outcome (replay-stable through the \
         live path)",
    );
    // And it's a non-trivial outcome (the strike actually connected — else determinism is vacuous).
    assert!(
        first.1 >= 1,
        "precondition: the deterministic run actually connects (variance 0 + a zero-Fight target)",
    );
}
