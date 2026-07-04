//! GTW-526 (child of GTW-41) — the SUPPRESSION CORE: a ganger fired near by an OPPONENT
//! is suppressed (a worse reactor that auto-drops behind cover), symmetric across both
//! factions, cleared at its own turn-start. Proven on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band runtime (the gtw468
//! reaction-harness idiom — the sim crate cannot dep on `gdtf_test_utils` without a
//! dependency cycle, so it drives `SetupBattleRequested` against a `MinimalPlugins` +
//! `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app, the exact production wiring).
//!
//! The clauses under test:
//!
//! - **(a) producer** — an opposing `FireRequested` marks an IN-radius opposing ganger
//!   `Suppressed`, not an OUT-of-radius one, not a SAME-faction one.
//! - **(b) determinism** — a suppressed reactor does NOT interrupt AND consumes ZERO
//!   `ReactionRng` draws: after the tick the world's `ReactionRng` stream sits at its
//!   INITIAL position (identical first draw to a fresh stream from the same seed), while
//!   an unsuppressed control reactor DOES interrupt and advances the stream.
//! - **(c) clear cadence** — a suppressed unit stays suppressed through the opponent's
//!   turn and loses `Suppressed` at its OWN faction's `TurnStarted`.
//! - **(d) auto-stance** — a freshly-suppressed unit auto-drops (low cover → Prone, mid
//!   cover → Crouching, no cover → unchanged) with NO TU charged.
//! - **(e) idempotent refresh** — a second opposing shot on an already-suppressed unit
//!   emits `SuppressionApplied` ONCE (not twice) and does not stack.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MessageReader, MinimalPlugins, Resource},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    ArmorHardness, ArmorProtection, Faction, Position, Stance, StanceKind, Suppressed,
    SuppressionApplied, Tu,
    acts::{EndTurnRequested, FireRequested},
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::{CoverEntry, CoverHp, CoverLedger, Destroyed, HeightBand},
    ganger::{Cool, Direction, Facing, GangRegistry, Grit, Reflexes, Speed, Toughness},
    metric::{Cell, CellLevel, Level},
    rng::{BattleSeed, ReactionRng},
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    weapon::{FireMode, Wields},
};

/// An arbitrary (not shipped tuning) seed for the test battle's RNG streams.
const SEED: u64 = 0x5_0BBE_7526;

/// Gang `0` is the player; gang `1` is the enemy.
const PLAYER: u8 = 0;
const ENEMY: u8 = 1;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A `CombatTuning` with the given `suppression_radius` (leaving the rest default), plus a
/// forced reaction clamp `p_min == p_max == 1.0` so the (b) control reactor is GUARANTEED
/// to interrupt — the determinism proof needs a live draw in the control.
fn tuning_with_radius(radius: u8) -> gdtf_battle_sim::tuning::CombatTuning {
    use gdtf_battle_sim::tuning::{
        CombatTuning, ReactionCapBase, ReactionCapPerReactions, ReactionPMax, ReactionPMin,
        ReactionTuning, SuppressionRadius, SuppressionStabilityPenalty,
    };
    CombatTuning {
        reaction: ReactionTuning {
            cap_base:            ReactionCapBase::new(8.0),
            cap_per_reactions:   ReactionCapPerReactions::new(0.0),
            p_min:               ReactionPMin::new(1.0),
            p_max:               ReactionPMax::new(1.0),
            suppression_radius:  SuppressionRadius::new(radius),
            // The core-slice tests are agnostic to the shot-cone penalty (they assert the
            // component / RNG-draw / clear cadence, not stability); keep the default.
            suppression_penalty: SuppressionStabilityPenalty::default(),
        },
        ..Default::default()
    }
}

/// Build the FULL live-runtime harness (the gtw468 `battle_app` idiom) with the given
/// suppression radius, plus the persistent `Load` weapon/armor registries a
/// `MinimalPlugins` app has no `AssetServer` to load.
fn battle_app(radius: u8) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(tuning_with_radius(radius));
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path and settle it.
fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    for _ in 0..4 {
        app.update();
    }
}

/// A standing ganger at `at` of `faction`, facing `facing`, with an ample TU pool + a fat
/// HP/Wounds pool (so an incidental shot cannot down it before the assertions complete).
fn ganger(
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
        .reflexes(Reflexes::new(20.0))
        .cool(Cool::new(20.0))
        .grit(Grit::new(80.0))
        .toughness(Toughness::new(80.0))
        .build()
}

/// The entity of the (first) ganger of `faction`.
fn ganger_of(app: &mut App, faction: u8) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
}

/// Every ganger of `faction`, in spawn order.
fn gangers_of(app: &mut App, faction: u8) -> Vec<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    let mut out: Vec<Entity> = query
        .iter(world)
        .filter(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
        .collect();
    out.sort();
    out
}

/// Whether `entity` currently carries the `Suppressed` marker.
fn is_suppressed(app: &App, entity: Entity) -> bool {
    app.world().get::<Suppressed>(entity).is_some()
}

/// The current stance kind of `entity`.
fn stance_of(app: &App, entity: Entity) -> Option<StanceKind> {
    app.world().get::<Stance>(entity).map(|s| **s)
}

/// The current TU of `entity`.
fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

/// The `single`-mode `FireModeSpec` of `shooter` (resolved off its wielded RANGED weapon),
/// so a test can emit a real `FireRequested`.
fn single_mode(app: &mut App, shooter: Entity) -> gdtf_battle_sim::weapon::FireModeSpec {
    use bevy::ecs::relationship::RelationshipTarget as _;
    // Resolve shooter → the wielded entity carrying a FireMode (the ranged weapon; the melee
    // `fists` entity has none) → its single spec. A hand-rolled traversal via the world (the
    // test-body idiom).
    let world = app.world_mut();
    let wielded: Vec<Entity> = {
        let Some(wields) = world.get::<Wields>(shooter) else {
            unreachable!("the shooter wields weapons at setup");
        };
        wields.iter().collect()
    };
    let mode = wielded
        .into_iter()
        .find_map(|entity| world.get::<FireMode>(entity).map(FireMode::single));
    let Some(mode) = mode else {
        unreachable!("the shooter wields a ranged weapon carrying a FireMode");
    };
    mode
}

/// Step `app` a fixed number of ticks.
fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

/// Whether the active faction is currently `faction`'s.
fn active_faction_is(app: &App, faction: u8) -> bool {
    app.world()
        .get_resource::<gdtf_battle_sim::ActiveFaction>()
        .is_some_and(|active| ***active == faction)
}

/// Run `app.update()` until the active faction is `faction` (crossing turn boundaries as
/// the enemy AI ends its turn back), or a generous tick budget is exhausted. Bounded.
fn run_until_active(app: &mut App, faction: u8) {
    for _ in 0..96 {
        app.update();
        if active_faction_is(app, faction) {
            return;
        }
    }
}

/// A test-local recorder of every `SuppressionApplied` observed across the run — so the
/// (e) idempotent-refresh count survives the one-update message lifetime.
#[derive(Resource, Default)]
struct AppliedLog {
    /// One entry per `SuppressionApplied` emitted: the cell it fired for.
    cells:   Vec<CellLevel>,
    /// One entry per `SuppressionApplied` emitted: the pinned ganger it carried (GTW-572 —
    /// the field the combat log's suppression line resolves to a name).
    gangers: Vec<bevy::prelude::Entity>,
}

/// Drain `SuppressionApplied` into the `AppliedLog` recorder.
fn record_applied(
    mut msgs: MessageReader<SuppressionApplied>,
    mut log: bevy::prelude::ResMut<AppliedLog>,
) {
    for msg in msgs.read() {
        log.cells.push(msg.at);
        log.gangers.push(msg.ganger);
    }
}

/// Whether some recorded `SuppressionApplied` carried `ganger` (GTW-572 — the signal names
/// the pinned unit, not just its cell).
fn applied_carried_ganger(app: &App, ganger: bevy::prelude::Entity) -> bool {
    app.world()
        .get_resource::<AppliedLog>()
        .is_some_and(|log| log.gangers.contains(&ganger))
}

/// Add the `SuppressionApplied` recorder to the app (after `BattleSimPlugin`, so the
/// buffer exists).
fn with_applied_log(app: &mut App) {
    app.init_resource::<AppliedLog>();
    app.add_systems(bevy::app::Update, record_applied);
}

/// How many `SuppressionApplied` signals fired for `cell` across the run.
fn applied_count_for(app: &App, cell: CellLevel) -> usize {
    app.world()
        .get_resource::<AppliedLog>()
        .map_or(0, |log| log.cells.iter().filter(|c| **c == cell).count())
}

/// A cover prototype at the given `band` (the test terrain registry ships only a LOW
/// cover and a HIGH wall, so a MID-band piece is seeded directly into the ledger from the
/// test body).
const fn cover_at_band(band: HeightBand) -> CoverEntry {
    CoverEntry {
        current_hp:       CoverHp::new(30),
        max_hp:           CoverHp::new(30),
        height_band:      band,
        armor_protection: ArmorProtection::new(2),
        armor_hardness:   ArmorHardness::new(1),
        destroyed:        Destroyed::new(false),
    }
}

/// A MID-band cover prototype seeded directly into the ledger.
const fn mid_cover() -> CoverEntry {
    cover_at_band(HeightBand::Mid)
}

/// A LOW-band cover prototype seeded directly into the ledger.
const fn low_cover() -> CoverEntry {
    cover_at_band(HeightBand::Low)
}

/// Seed a cover entry into the battle's `CoverLedger` at `cell` (test-body idiom).
fn seed_cover(app: &mut App, cell: CellLevel, entry: CoverEntry) {
    let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() else {
        unreachable!("BattleSimPlugin inserts the CoverLedger at setup");
    };
    ledger.insert(cell, entry);
}

// === (a) — the producer suppresses an IN-radius opposing ganger, not out-of-radius, not
// same-faction. ===

#[test]
fn producer_suppresses_in_radius_opposing_ganger_only() {
    // Radius 1: the shot's target cell + its Moore-8 ring are suppressed.
    let mut app = battle_app(1);
    with_applied_log(&mut app);

    // The enemy shooter sits west; the target cell is (8,5). An opposing PLAYER stands ON
    // the target (in radius), another PLAYER stands far away (out of radius), and a SECOND
    // ENEMY stands adjacent to the target (same faction as the shooter — never suppressed).
    let target = ground(8, 5);
    let in_radius_player = target; // radius 1 includes the target cell (Chebyshev 0)
    let out_of_radius_player = ground(20, 20);
    let same_faction_near = ground(8, 6); // Chebyshev 1 of target, but ENEMY faction
    let situation = SituationBuilder::new()
        .with_gangers([
            ganger(in_radius_player, PLAYER, Direction::West),
            ganger(out_of_radius_player, PLAYER, Direction::West),
            ganger(ground(4, 5), ENEMY, Direction::East),
            ganger(same_faction_near, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let players = gangers_of(&mut app, PLAYER);
    let enemies = gangers_of(&mut app, ENEMY);
    assert_eq!(players.len(), 2, "two players spawned");
    assert_eq!(enemies.len(), 2, "two enemies spawned");
    let Some(shooter) = ganger_of(&mut app, ENEMY) else {
        unreachable!("an enemy shooter spawned");
    };
    // Identify the in-radius vs out-of-radius player by position.
    let in_player = players
        .iter()
        .copied()
        .find(|e| app.world().get::<Position>(*e).map(|p| **p) == Some(in_radius_player));
    let out_player = players
        .iter()
        .copied()
        .find(|e| app.world().get::<Position>(*e).map(|p| **p) == Some(out_of_radius_player));
    let same_faction = enemies
        .iter()
        .copied()
        .find(|e| app.world().get::<Position>(*e).map(|p| **p) == Some(same_faction_near));
    let (Some(in_player), Some(out_player), Some(same_faction)) =
        (in_player, out_player, same_faction)
    else {
        unreachable!("the three tagged gangers resolve by position");
    };

    // Emit a real enemy FireRequested aimed at the target cell.
    let mode = single_mode(&mut app, shooter);
    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        Cell::new(8, 5),
        Level::new(0),
    ));
    // Settle: dispatch_fire (early) → apply_suppression (.after(dispatch_fire)) → the
    // Commands insert flushes → next tick the Suppressed component is visible.
    step(&mut app, 3);

    assert!(
        is_suppressed(&app, in_player),
        "(a) the IN-radius opposing player is Suppressed",
    );
    assert!(
        !is_suppressed(&app, out_player),
        "(a) the OUT-of-radius opposing player is NOT suppressed",
    );
    assert!(
        !is_suppressed(&app, same_faction),
        "(a) a SAME-faction ganger (the shooter's own gang) is NEVER suppressed",
    );
    assert!(
        !is_suppressed(&app, shooter),
        "(a) the shooter never suppresses itself",
    );
    // GTW-572: the fresh SuppressionApplied signal CARRIES the pinned ganger (the field the
    // combat log's suppression line resolves to a name), not just its cell.
    assert!(
        applied_carried_ganger(&app, in_player),
        "GTW-572: the SuppressionApplied signal must carry the freshly-pinned ganger entity",
    );
}

// === (b) — a suppressed reactor does NOT interrupt AND consumes ZERO ReactionRng draws;
// an unsuppressed control DOES interrupt and advances the stream. ===

#[test]
fn suppressed_reactor_consumes_zero_reaction_rng_draws() {
    // The first draw of a FRESH ReactionRng from this seed — the position the world's
    // stream must still sit at when the suppressed reactor draws nothing.
    let fresh_first_draw = ReactionRng::from_root(BattleSeed::new(SEED)).next_u64();

    // Build a scenario where a ganger FIRING trips the FireDeclaration act-in-LOS surface,
    // and an opposing watcher in LOS would interrupt (forced p == 1.0). One reactor, one
    // actor — so the ONLY possible ReactionRng draw is the watcher's interrupt roll.
    let build = |suppress_watcher: bool| {
        // Radius 0 so the actor's own fire suppresses only its target cell (an empty cell),
        // never the watcher — the watcher's suppression is set explicitly below, isolating
        // the C3 gate from the producer.
        let mut app = battle_app(0);
        let situation = SituationBuilder::new()
            .with_gangers([
                // The ACTOR: an enemy at (6,5) firing east (away from the watcher) — the
                // FireDeclaration is the act the watcher reacts to.
                ganger(ground(6, 5), ENEMY, Direction::East),
                // The WATCHER: a player at (4,5) facing east, in LOS + arc + range of the
                // actor's cell.
                ganger(ground(4, 5), PLAYER, Direction::East),
            ])
            .build_with_gangs();
        drive_setup(&mut app, situation);
        let (Some(actor), Some(watcher)) =
            (ganger_of(&mut app, ENEMY), ganger_of(&mut app, PLAYER))
        else {
            unreachable!("one enemy actor + one player watcher spawned");
        };
        if suppress_watcher {
            // Suppress the WATCHER directly (test-body idiom) BEFORE it can react — the C3
            // reaction-gate must skip it with zero RNG draws.
            app.world_mut().entity_mut(watcher).insert(Suppressed::new(
                gdtf_battle_sim::ganger::SuppressorCell::new(ground(6, 5)),
            ));
            step(&mut app, 1); // no-op stance drop (no cover), keeps the marker
        }
        // The actor FIRES (a real FireRequested at a far cell, away from the watcher) — this
        // trips the FireDeclaration surface reaction_trigger reads next tick.
        let mode = single_mode(&mut app, actor);
        app.world_mut().write_message(FireRequested::new(
            actor,
            mode,
            Cell::new(9, 5),
            Level::new(0),
        ));
        step(&mut app, 4);
        let watcher_tu = tu_of(&app, watcher);
        // The world's ReactionRng position — its NEXT draw (mutates the world's stream, but
        // the app is discarded right after, so it is a safe read of the current position).
        let next_draw = app
            .world_mut()
            .get_resource_mut::<ReactionRng>()
            .map(|mut r| r.next_u64());
        (watcher, watcher_tu, next_draw)
    };

    // CONTROL: watcher NOT suppressed → it interrupts → 1 ReactionRng draw consumed → the
    // stream has ADVANCED past the fresh first draw.
    let (_control_watcher, control_tu, control_next) = build(false);
    let Some(control_tu) = control_tu else {
        unreachable!("the control watcher persists");
    };
    let Some(control_next) = control_next else {
        unreachable!("the ReactionRng stream is present in the control");
    };

    // SUPPRESSED: watcher IS suppressed → it does NOT interrupt → 0 ReactionRng draws → the
    // stream still sits at its INITIAL position (its next draw == the fresh first draw).
    let (_supp_watcher, supp_tu, supp_next) = build(true);
    let Some(supp_tu) = supp_tu else {
        unreachable!("the suppressed watcher persists");
    };
    let Some(supp_next) = supp_next else {
        unreachable!("the ReactionRng stream is present in the suppressed run");
    };

    assert_eq!(
        supp_next, fresh_first_draw,
        "(b) the suppressed reactor consumed ZERO ReactionRng draws — the world stream still \
         sits at its initial position (next draw == a fresh stream's first draw)",
    );
    assert_ne!(
        control_next, fresh_first_draw,
        "(b) precondition: the unsuppressed control reactor DID draw (its interrupt roll \
         advanced the stream past the fresh first draw) — else the zero-draw claim is vacuous",
    );
    // Structural corroboration through TU: the control watcher spent fire TU on its
    // interrupt; the suppressed watcher did not interrupt, so its TU is untouched.
    assert!(
        control_tu < supp_tu,
        "(b) the control watcher's TU debits on its interrupt shot ({control_tu}); the \
         suppressed watcher never fires, so its TU is higher ({supp_tu})",
    );
}

// === (c) — the clear cadence: a unit stays suppressed through the opponent's turn and
// clears at its OWN faction's TurnStarted. ===

#[test]
fn suppression_clears_at_the_suppressed_units_own_turn_start() {
    let mut app = battle_app(1);
    // A player unit at the target of an enemy shot (player turn is active at setup, but we
    // drive the shot directly). The enemy shooter is west.
    let target = ground(8, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            ganger(target, PLAYER, Direction::West),
            ganger(ground(4, 5), ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("one player + one enemy spawned");
    };

    // Suppress the PLAYER via a real enemy fire on its cell.
    let mode = single_mode(&mut app, enemy);
    app.world_mut().write_message(FireRequested::new(
        enemy,
        mode,
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);
    assert!(
        is_suppressed(&app, player),
        "(c) precondition: the player is suppressed by the enemy shot",
    );

    // End the PLAYER turn → the ENEMY turn begins (a TurnStarted { now_active: ENEMY }). The
    // player unit must STAY suppressed — it is now in its opponent's turn.
    app.world_mut().write_message(EndTurnRequested);
    step(&mut app, 3);
    assert!(
        active_faction_is(&app, ENEMY),
        "(c) precondition: the enemy turn is active after the player ends its turn",
    );
    assert!(
        is_suppressed(&app, player),
        "(c) the player stays suppressed through the ENEMY's turn (cleared only at its OWN \
         turn-start, not the opponent's)",
    );

    // The enemy AI runs its turn and ends it back to the player → a TurnStarted { now_active:
    // PLAYER }. The player unit's suppression must CLEAR at that own-faction boundary.
    run_until_active(&mut app, PLAYER);
    assert!(
        active_faction_is(&app, PLAYER),
        "(c) precondition: control cycled back to the player turn",
    );
    // A settle tick so the same-frame reset_suppression (.after(dispatch_end_turn)) has run.
    step(&mut app, 1);
    assert!(
        !is_suppressed(&app, player),
        "(c) the player's suppression clears at ITS OWN (player) turn-start",
    );
}

// === (d) — auto-stance: low cover → Prone, mid cover → Crouching, no cover → unchanged;
// NO TU charged. ===

#[test]
fn auto_stance_drops_behind_cover_without_charging_tu() {
    // Three separate suppressions, each in its own app so the cover geometry is clean.
    // The suppressor sits WEST of the unit, so the cover "one step toward the suppressor"
    // is one cell WEST of the unit.
    let run = |cover: Option<CoverEntry>| -> (Option<StanceKind>, Option<u8>) {
        let mut app = battle_app(0);
        let unit_cell = ground(8, 5);
        let cover_cell = ground(7, 5); // one step WEST (toward the suppressor at (4,5))
        let situation = SituationBuilder::new()
            .with_gangers([
                ganger(unit_cell, PLAYER, Direction::East),
                ganger(ground(4, 5), ENEMY, Direction::East),
            ])
            .build_with_gangs();
        drive_setup(&mut app, situation);
        if let Some(entry) = cover {
            seed_cover(&mut app, cover_cell, entry);
        }
        let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
        else {
            unreachable!("one player + one enemy spawned");
        };
        let tu_before = tu_of(&app, player);
        // Enemy fires on the player's cell → suppresses it (radius 0 = the target cell).
        let mode = single_mode(&mut app, enemy);
        app.world_mut().write_message(FireRequested::new(
            enemy,
            mode,
            Cell::new(8, 5),
            Level::new(0),
        ));
        step(&mut app, 3);
        assert!(
            is_suppressed(&app, player),
            "auto-stance precondition: the player is suppressed",
        );
        let stance_after = stance_of(&app, player);
        let tu_after = tu_of(&app, player);
        // The auto-stance drop must NOT charge TU (a reflexive duck, no action spent).
        assert_eq!(
            tu_before, tu_after,
            "(d) the auto-stance drop charges NO TU (before {tu_before:?} == after \
             {tu_after:?})",
        );
        (stance_after, tu_after)
    };

    let (low_stance, _) = run(Some(low_cover()));
    assert_eq!(
        low_stance,
        Some(StanceKind::Prone),
        "(d) LOW cover → the unit auto-drops to Prone",
    );

    let (mid_stance, _) = run(Some(mid_cover()));
    assert_eq!(
        mid_stance,
        Some(StanceKind::Crouching),
        "(d) MID cover → the unit auto-drops to Crouching",
    );

    let (none_stance, _) = run(None);
    assert_eq!(
        none_stance,
        Some(StanceKind::Standing),
        "(d) NO adjacent cover → the stance is UNCHANGED (still Standing, the spawn posture)",
    );
}

// === (e) — idempotent refresh: a second opposing shot on an already-suppressed unit emits
// SuppressionApplied ONCE, not twice. ===

#[test]
fn idempotent_refresh_emits_suppression_applied_once() {
    let mut app = battle_app(1);
    with_applied_log(&mut app);
    let target = ground(8, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            ganger(target, PLAYER, Direction::West),
            ganger(ground(4, 5), ENEMY, Direction::East),
            ganger(ground(4, 6), ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let Some(player) = ganger_of(&mut app, PLAYER) else {
        unreachable!("a player spawned");
    };
    let enemies = gangers_of(&mut app, ENEMY);
    let (Some(&first_shooter), Some(&second_shooter)) = (enemies.first(), enemies.get(1)) else {
        unreachable!("two enemy shooters spawned");
    };

    // First shot: a FRESH suppression → one SuppressionApplied for the player's cell.
    let mode1 = single_mode(&mut app, first_shooter);
    app.world_mut().write_message(FireRequested::new(
        first_shooter,
        mode1,
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);
    assert!(
        is_suppressed(&app, player),
        "(e) precondition: the first shot suppresses the player",
    );

    // Second shot (a DIFFERENT shooter, so it is not de-duplicated as the same fire): the
    // player is ALREADY suppressed → an idempotent REFRESH → NO second SuppressionApplied.
    let mode2 = single_mode(&mut app, second_shooter);
    app.world_mut().write_message(FireRequested::new(
        second_shooter,
        mode2,
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);

    assert!(
        is_suppressed(&app, player),
        "(e) the player is still suppressed after the refresh",
    );
    assert_eq!(
        applied_count_for(&app, target),
        1,
        "(e) SuppressionApplied fired exactly ONCE for the player's cell (a re-application of \
         an already-suppressed unit is an idempotent refresh — no second FCT signal)",
    );
}
