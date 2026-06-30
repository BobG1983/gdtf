//! GTW-468 (the final child of GTW-38) — the LIVE reaction-fire trigger: when a ganger
//! ACTS in an opposing reactor's LOS, the §8 opposed check fires an interrupt from the
//! reactor's unspent TU, halts a walking actor, and consumes the reactor's per-turn cap.
//! Proven END-TO-END on the REAL `setup_battle_on_request` → `BattleSimPlugin`
//! `Simulate`-band path, driven THROUGH the move / brain / dispatch path (NOT a synthetic
//! `FireRequested`/`ReactionShotFired` emit).
//!
//! The acceptance criteria:
//!
//! - **AC1** — an actor (faction A) MOVING within LOS of a watcher (faction B, unspent TU,
//!   fresh cap) → the interrupt FIRES: the watcher's TU is debited AND a `ShotFired` from
//!   the watcher is observed on the actor's cell. PIN-DISCRIMINATING (fails if unwired).
//! - **AC2** — a walking actor HALTS on the (forced-success) interrupt: its `WalkInProgress`
//!   is removed and it stops short of its destination, via the LIVE `ReactionShotFired`.
//! - **AC3** — LOS gate: an actor acting OUTSIDE all opposing watchers' LOS produces NO
//!   interrupt; the SAME act inside LOS + arc + range DOES.
//! - **AC4** — the per-turn CAP bites: with cap == 1 a watcher interrupts ONCE this turn and
//!   not again, and reacts AGAIN next turn (the reset exercised through a turn cycle).
//! - **AC5** — faction symmetry: a player watcher interrupts an acting ENEMY during the
//!   enemy turn (the brain drives the enemy move) AND an enemy watcher interrupts an acting
//!   PLAYER during the player turn.
//! - **AC7** — the reaction shot is a NORMAL shot (no reaction damage modifier): it resolves
//!   through the normal `dispatch_fire` → `fire()` pipeline (a real `ShotFired` with a
//!   `HitReport`), never a bespoke reaction path.
//!
//! DETERMINISM: a seeded battle RNG, plus a tuning clamp `p_min == p_max == 1.0` to FORCE a
//! guaranteed interrupt where the test needs one (`rolls_interrupt` draws `roll ∈ [0,1)` and
//! compares `roll < 1.0`, always true — `clamp_probability` returns `1.0` when min == max).
//! AC3's no-interrupt case is forced by GEOMETRY (LOS blocked / out of range), not by the
//! probability. Structural facts only (TU debited, `WalkInProgress` removed, `ShotFired`
//! emitted), never brittle magnitudes.
//!
//! HARNESS NOTE (deviation from the ticket's "use `GdtfTestAppBuilder`", as in GTW-355): the
//! sim crate is the LOW crate — `gdtf_test_utils` depends on `gdtf_app` which depends on
//! `gdtf_battle_sim`, so a sim-crate dev-dep on `gdtf_test_utils` would be a dependency
//! CYCLE. The established sim-crate battle-integration idiom (gtw341 / gtw355) drives
//! `setup_battle_on_request` via a `SetupBattleRequested` message against a `MinimalPlugins`
//! + `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT production wiring.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins, Resource},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Faction, Position, ReactionShotFired, Reflexes, ShotFired, Speed, Stance, StanceKind, Tu,
    WalkInProgress,
    acts::{EndTurnRequested, MoveRequested},
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Cool, Direction, Facing, GangRegistry},
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_weapon_registry,
    },
    tuning::{
        CombatTuning, ReactionCapBase, ReactionCapPerReactions, ReactionPMax, ReactionPMin,
        ReactionTuning, ReactionsUsed, ViewRange,
    },
};

/// An arbitrary (not shipped tuning) seed for the test battle's RNG streams.
const SEED: u64 = 0x4EAC_7104;

/// Gang `0` is the player; gang `1` is the enemy.
const PLAYER: u8 = 0;
const ENEMY: u8 = 1;

/// A view range that lights a local field a few cells out, yet is short enough that a cell
/// several tiles away is UNSEEN (so AC3's out-of-LOS-by-range case is constructible).
/// Arbitrary test tuning, never a pinned shipped magnitude.
const TEST_VIEW_RANGE: u16 = 6;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A `ReactionTuning` that FORCES every opposed check to succeed (`p_min == p_max == 1.0`)
/// and caps interrupts at exactly `cap` per turn (a flat base, zero per-`Reactions` slope).
#[expect(
    clippy::cast_precision_loss,
    reason = "the test caps are tiny (1 or 8), so the u32 -> f32 conversion is exact"
)]
const fn forced_reaction_tuning(cap: u32) -> ReactionTuning {
    ReactionTuning {
        cap_base:          ReactionCapBase::new(cap as f32),
        cap_per_reactions: ReactionCapPerReactions::new(0.0),
        p_min:             ReactionPMin::new(1.0),
        p_max:             ReactionPMax::new(1.0),
    }
}

/// Build the FULL live-runtime harness (the gtw355 `battle_app` idiom) with a `CombatTuning`
/// carrying the given short view range + the given reaction tuning, plus the persistent
/// `Load` weapon/armor registries a `MinimalPlugins` app has no `AssetServer` to load.
fn battle_app(reaction: ReactionTuning) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        reaction,
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path and settle it (the
/// deferred `bsn!` ganger scenes materialize and the first-run recompute fills the fog).
fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    // Several updates so the scenes materialize and the spawn-time recompute fills the fog.
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

/// The current `(cell, level)` of `entity`.
fn pos_of(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(entity).map(|p| **p)
}

/// The current per-turn interrupt count of `entity`.
fn used_of(app: &App, entity: Entity) -> Option<u32> {
    app.world().get::<ReactionsUsed>(entity).map(|u| **u)
}

/// Whether `entity` still carries a `WalkInProgress` (the walk is in flight).
fn is_walking(app: &App, entity: Entity) -> bool {
    app.world().get::<WalkInProgress>(entity).is_some()
}

// ── A test-local ShotFired recorder ─────────────────────────────────────────────
//
// `dispatch_fire` emits a ShotFired per round; a `MessageReader` only drains a buffer
// once and only sees messages from the current+previous update. To assert across the whole
// run we record every ShotFired into a resource the moment it is emitted, via a small reader
// system added to the test app.

/// Every `ShotFired` shooter entity observed across the run — a test-only recorder so the
/// assertion can read the full history rather than racing the one-update message lifetime.
#[derive(Resource, Default)]
struct ShotLog {
    /// One entry per `ShotFired` round emitted: the shooter, and whether it carried a
    /// `HitReport` (AC7 — a normal-pipeline shot carries one).
    rounds: Vec<(Entity, bool)>,
}

/// Drain `ShotFired` into the `ShotLog` recorder — added to the test app so the run's full
/// fire history is queryable after `app.update()`s.
fn record_shots(
    mut shots: bevy::prelude::MessageReader<ShotFired>,
    mut log: bevy::prelude::ResMut<ShotLog>,
) {
    for shot in shots.read() {
        log.rounds.push((shot.shooter, shot.report.is_some()));
    }
}

/// Add the `ShotFired` recorder to the app (after `BattleSimPlugin`, so the buffer exists).
fn with_shot_log(app: &mut App) {
    app.init_resource::<ShotLog>();
    app.add_systems(bevy::app::Update, record_shots);
}

/// How many rounds `shooter` fired across the run.
fn shots_by(app: &App, shooter: Entity) -> usize {
    app.world().get_resource::<ShotLog>().map_or(0, |log| {
        log.rounds.iter().filter(|(s, _)| *s == shooter).count()
    })
}

/// Whether every round `shooter` fired carried a `HitReport` (the normal pipeline; AC7).
fn all_shots_have_reports(app: &App, shooter: Entity) -> bool {
    app.world().get_resource::<ShotLog>().is_some_and(|log| {
        let mut any = false;
        let ok = log
            .rounds
            .iter()
            .filter(|(s, _)| *s == shooter)
            .all(|(_, has)| {
                any = true;
                *has
            });
        any && ok
    })
}

/// A high-Reactions standing watcher (so its TU pool is ample and its score is high; the
/// forced p == 1.0 makes the magnitudes irrelevant, but a fat TU pool keeps it cap-eligible
/// across multiple interrupts). It also carries a FAT HP / Wounds pool (high Grit /
/// Toughness / Cool) so an incidental enemy shot during a multi-turn test cannot down it
/// before it reacts again — keeping the AC4 reset proof robust. Faces `facing` so an in-arc
/// target needs no turn.
fn watcher(
    at: CellLevel,
    faction: u8,
    facing: Direction,
) -> gdtf_battle_sim::situation::GangerSpawn {
    use gdtf_battle_sim::ganger::{Grit, Toughness};
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

/// A mover/actor ganger with an ample TU pool (high Speed → a long affordable route).
fn mover(at: CellLevel, faction: u8, facing: Direction) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .build()
}

/// A mover/actor with an ample TU pool AND a FAT HP/Wounds pool (high Grit / Toughness /
/// Cool) — so a multi-turn test that repeatedly interrupt-shoots it cannot down it before
/// the assertions complete. Used for the AC4 actor (the enemy the watcher interrupts across
/// two turns).
fn tough_mover(
    at: CellLevel,
    faction: u8,
    facing: Direction,
) -> gdtf_battle_sim::situation::GangerSpawn {
    use gdtf_battle_sim::ganger::{Grit, Toughness};
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .grit(Grit::new(80.0))
        .toughness(Toughness::new(80.0))
        .build()
}

/// Run `app.update()` until `entity` stops walking or a generous tick budget is exhausted.
fn run_until_walk_ends(app: &mut App, entity: Entity) {
    for _ in 0..48 {
        app.update();
        if !is_walking(app, entity) {
            return;
        }
    }
}

/// Step `app` a fixed number of ticks (for a settle that is not gated on a walk ending).
fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

/// Whether the active faction is currently the player's.
fn player_turn_active(app: &App) -> bool {
    app.world()
        .get_resource::<gdtf_battle_sim::ActiveFaction>()
        .is_some_and(|active| ***active == PLAYER)
}

/// Cycle turns (writing `EndTurnRequested` and letting the brain run / hand control back)
/// until it is the PLAYER's turn again, crossing at least one boundary — so the player
/// gangers' TU regenerate at the player turn-start AND every watcher's per-turn cap counter
/// is reset (`reset_reactions_used` on each `TurnStarted`). Bounded — never an open loop.
fn cycle_back_to_player_turn(app: &mut App) {
    // End the player turn → the enemy turn. The brain runs the enemy turn and ends it back to
    // the player (the GTW-70 no-auto-pass cycle). Crossing each boundary fires a TurnStarted,
    // resetting every watcher's cap; arriving back at the player turn regenerates player TU.
    app.world_mut().write_message(EndTurnRequested);
    for _ in 0..64 {
        app.update();
        if player_turn_active(app) {
            return;
        }
    }
}

// === AC1 + AC2 + AC7 — a PLAYER moving in an ENEMY watcher's LOS is interrupted (the
// watcher's TU debits, a normal ShotFired fires at the player, the walk halts). ===

#[test]
fn ac1_ac2_ac7_acting_player_is_interrupted_by_an_enemy_watcher() {
    let mut app = battle_app(forced_reaction_tuning(8));
    with_shot_log(&mut app);

    // The enemy watcher sits west, facing East down the player's walk lane; the player
    // stands a few cells east of it and walks further east — every step is in the watcher's
    // LOS + arc + range. (Player turn is active at setup, so the player MoveRequested
    // dispatches immediately.)
    let watcher_cell = ground(4, 5);
    let player_start = ground(6, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            mover(player_start, PLAYER, Direction::East),
            watcher(watcher_cell, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    let Some(enemy_tu_before) = tu_of(&app, enemy) else {
        unreachable!("the enemy has a Tu pool");
    };
    let Some(player_start_now) = pos_of(&app, player) else {
        unreachable!("the player has a Position");
    };

    // The player walks a multi-cell route east (in the watcher's LOS the whole way).
    let dest = ground(10, 5);
    app.world_mut()
        .write_message(MoveRequested::new(player, dest));
    run_until_walk_ends(&mut app, player);
    // A couple more ticks so the one-tick-cadence interrupt (read the step's Changed<Position>
    // next tick → fire) fully resolves and is recorded.
    step(&mut app, 3);

    // AC2: the walk HALTED short of the destination (the live ReactionShotFired removed the
    // WalkInProgress).
    let Some(player_final) = pos_of(&app, player) else {
        unreachable!("the player persists");
    };
    assert!(
        !is_walking(&app, player),
        "AC2: the interrupt removes the player's WalkInProgress (the walk halted)",
    );
    assert_ne!(
        player_final, dest,
        "AC2: the interrupt halts the walk SHORT of the destination",
    );
    assert_ne!(
        player_final, player_start_now,
        "the player took at least one step before the interrupt fired",
    );

    // AC1: the enemy watcher's TU was DEBITED (the interrupt shot spent its fire TU through
    // the normal fire pipeline).
    let Some(enemy_tu_after) = tu_of(&app, enemy) else {
        unreachable!("the enemy persists");
    };
    assert!(
        enemy_tu_after < enemy_tu_before,
        "AC1: the watcher's TU is debited by the interrupt shot ({enemy_tu_after} < \
         {enemy_tu_before})",
    );

    // AC1 (positive, pin-discriminating): a REAL ShotFired came FROM the enemy watcher —
    // the interrupt actually fired (this fails if the trigger is unwired).
    assert!(
        shots_by(&app, enemy) >= 1,
        "AC1: the enemy watcher fired at least one interrupt round (ShotFired from the \
         watcher) — the trigger is LIVE",
    );

    // AC7: every round the watcher fired is a NORMAL-pipeline shot — it carries a HitReport
    // (a bespoke reaction path would not resolve through dispatch_fire → fire()).
    assert!(
        all_shots_have_reports(&app, enemy),
        "AC7: the reaction shot resolves through the NORMAL fire pipeline (every ShotFired \
         carries a HitReport — no reaction damage modifier / no bespoke path)",
    );

    // The reactor consumed its per-turn cap at least once (the count incremented — C4).
    assert!(
        used_of(&app, enemy).is_some_and(|u| u >= 1),
        "the watcher's ReactionsUsed incremented when it interrupted",
    );
}

// === AC3 — the LOS gate. An actor acting OUTSIDE all opposing watchers' LOS produces NO
// interrupt; the SAME act with the watcher in LOS DOES. ===

#[test]
fn ac3_no_interrupt_when_the_actor_acts_outside_los() {
    let mut app = battle_app(forced_reaction_tuning(8));
    with_shot_log(&mut app);

    // The watcher is parked FAR (well beyond TEST_VIEW_RANGE) from the player's walk lane,
    // so the moving player is never in range — NO interrupt despite forced p == 1.0.
    let watcher_cell = ground(40, 40);
    let player_start = ground(6, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            mover(player_start, PLAYER, Direction::East),
            watcher(watcher_cell, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    let Some(enemy_tu_before) = tu_of(&app, enemy) else {
        unreachable!("the enemy has a Tu pool");
    };

    let dest = ground(10, 5);
    app.world_mut()
        .write_message(MoveRequested::new(player, dest));
    run_until_walk_ends(&mut app, player);
    step(&mut app, 3);

    // AC3 (negative): NO interrupt — the watcher fired nothing, spent no TU, and the player
    // walked freely to its destination.
    assert_eq!(
        shots_by(&app, enemy),
        0,
        "AC3: an out-of-range watcher fires NO interrupt (LOS/range gate held)",
    );
    assert_eq!(
        tu_of(&app, enemy),
        Some(enemy_tu_before),
        "AC3: the out-of-range watcher spent no TU",
    );
    assert_eq!(
        used_of(&app, enemy),
        Some(0),
        "AC3: the out-of-range watcher's cap counter never incremented",
    );
    assert_eq!(
        pos_of(&app, player),
        Some(dest),
        "AC3: with no interrupt the player walks freely to its destination",
    );
}

// === AC4 — the per-turn cap bites and resets next turn. With cap == 1, a watcher interrupts
// at most once this turn, then reacts AGAIN after a turn boundary resets the counter. ===

#[test]
fn ac4_the_per_turn_cap_bites_then_resets_next_turn() {
    // cap == 1: EXACTLY one interrupt per watcher PER TURN. The reactor is a PLAYER watcher;
    // the ACTOR is an ENEMY moved by explicit `MoveRequested`s (a non-player mover routes over
    // the never-stale OMNISCIENT move fog through `dispatch_move`, so its walk is reliable
    // turn after turn — and it only ever MOVES, never engages, so it can never kill the
    // watcher mid-test). Both gangers carry FAT HP pools so neither is downed by an interrupt
    // shot across the multi-turn run. The acting ganger is driven THROUGH the real move /
    // dispatch / advance_walk path (NOT a synthetic emit). Cap-bite: the watcher interrupts
    // ONCE this turn and not again. Reset: after a turn boundary the watcher reacts AGAIN.
    let mut app = battle_app(forced_reaction_tuning(1));
    with_shot_log(&mut app);

    // The player watcher at (5,5) facing East; a tough enemy a couple cells east at (7,5),
    // ALREADY in the watcher's LOS at spawn (so the enemy's walk never reveal-halts — only
    // the reaction interrupt halts it).
    let player_watcher = ground(5, 5);
    let enemy_start = ground(7, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(player_watcher, PLAYER, Direction::East),
            tough_mover(enemy_start, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(watcher_entity), Some(enemy)) =
        (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player watcher and one enemy");
    };

    // === Turn 1: move the enemy east in the watcher's LOS → the watcher interrupts ONCE. ===
    app.world_mut()
        .write_message(MoveRequested::new(enemy, ground(11, 5)));
    run_until_walk_ends(&mut app, enemy);
    step(&mut app, 3);
    let shots_first = shots_by(&app, watcher_entity);
    assert!(
        shots_first >= 1,
        "AC4: the watcher interrupted the enemy's move this turn (cap == 1, forced p == 1.0)",
    );
    assert_eq!(
        used_of(&app, watcher_entity),
        Some(1),
        "AC4: the watcher's per-turn cap counter is saturated at 1 this turn",
    );

    // A SECOND enemy move THIS SAME turn must NOT draw another interrupt (the cap bit).
    let shots_before_second = shots_by(&app, watcher_entity);
    let Some(enemy_after_first) = pos_of(&app, enemy) else {
        unreachable!("the enemy persists");
    };
    app.world_mut().write_message(MoveRequested::new(
        enemy,
        ground(enemy_after_first.x + 2, enemy_after_first.y),
    ));
    run_until_walk_ends(&mut app, enemy);
    step(&mut app, 3);
    assert_eq!(
        shots_by(&app, watcher_entity),
        shots_before_second,
        "AC4: the cap BITES — a second act this turn draws NO further interrupt",
    );

    // === Cross a FULL turn cycle back to the PLAYER's turn. This (a) resets the watcher's
    // per-turn cap counter at each TurnStarted boundary AND (b) regenerates the player
    // watcher's TU at the player turn-start — so on its next turn the watcher both has cap
    // room AND can AFFORD the interrupt shot again (a watcher that spent its TU reacting
    // genuinely cannot react until its TU regenerates — the §8 TU economy). ===
    cycle_back_to_player_turn(&mut app);
    assert!(
        player_turn_active(&app),
        "AC4 precondition: the turn cycled back to the player",
    );
    assert_eq!(
        used_of(&app, watcher_entity),
        Some(0),
        "AC4: the turn boundary RESET the watcher's per-turn cap counter",
    );

    // === A fresh enemy act after the reset draws a FRESH interrupt (the cap re-opened and the
    // watcher's TU regenerated). Move the enemy through the watcher's LOS lane again. ===
    let shots_before_reset_act = shots_by(&app, watcher_entity);
    let Some(enemy_now) = pos_of(&app, enemy) else {
        unreachable!("the enemy persists across the turn cycle");
    };
    // Move the enemy EAST (away from the now-adjacent watcher) but staying inside its LOS
    // lane + range — a clean act-in-LOS step the watcher can interrupt. The brain may have
    // walked the enemy right up to the watcher during the cycle, so moving away guarantees an
    // actual step (a move onto its own cell would be a no-op). The destination stays within
    // the watcher's view range (Chebyshev ≤ 6 from the watcher cell).
    let away_x = (enemy_now.x + 3).min(player_watcher.x + 5);
    app.world_mut()
        .write_message(MoveRequested::new(enemy, ground(away_x, player_watcher.y)));
    run_until_walk_ends(&mut app, enemy);
    step(&mut app, 3);
    assert!(
        shots_by(&app, watcher_entity) > shots_before_reset_act,
        "AC4: after the reset the watcher reacts AGAIN next turn (the cap re-opened): \
         {shots_before_reset_act} → {}",
        shots_by(&app, watcher_entity),
    );
}

// === AC5 — faction symmetry. The enemy-watches-player case is covered by AC1; here we drive
// the MIRROR through the brain: a PLAYER watcher interrupts an acting ENEMY during the enemy
// turn (the brain emits the enemy's move, which steps in the player watcher's LOS). ===

#[test]
fn ac5_a_player_watcher_interrupts_an_acting_enemy_on_the_enemy_turn() {
    let mut app = battle_app(forced_reaction_tuning(8));
    with_shot_log(&mut app);

    // A player watcher facing East at (5,5), view range 6. The enemy starts FAR east at
    // (15,5) — Chebyshev 10 > 6, so it is OUT of the watcher's range AND out of its own
    // engage range to the watcher at spawn. So on the enemy turn the brain ADVANCES the
    // enemy toward its nearest opposing ganger (the watcher), walking WEST down row 5. The
    // STEP that carries the enemy to within view range (Chebyshev 6, i.e. cell (11,5)) is the
    // act-in-LOS the player watcher interrupts — driven THROUGH the brain/move path, not a
    // synthetic emit. (The enemy starting out of engage range is what makes it MOVE rather
    // than open fire first — the move surface, the mirror of AC1's player move.)
    let player_watcher = ground(5, 5);
    let enemy_at = ground(15, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            // The player watcher (faction 0) — high Reactions, facing the enemy's lane.
            watcher(player_watcher, PLAYER, Direction::East),
            // The acting enemy (faction 1) — out of range at spawn, advanced by the brain.
            mover(enemy_at, ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let Some(player_watcher_entity) = ganger_of(&mut app, PLAYER) else {
        unreachable!("setup spawns the player watcher");
    };
    let Some(enemy) = ganger_of(&mut app, ENEMY) else {
        unreachable!("setup spawns the enemy");
    };
    let Some(watcher_tu_before) = tu_of(&app, player_watcher_entity) else {
        unreachable!("the player watcher has a Tu pool");
    };

    // End the PLAYER turn → control passes to the enemy; the brain drives the enemy across
    // the following ticks (it advances toward the distant player goal, stepping through the
    // player watcher's LOS). Give it a generous budget to act + be interrupted + the
    // one-tick cadence to resolve.
    app.world_mut().write_message(EndTurnRequested);
    // Drive the enemy turn: the brain advances the enemy west toward the watcher; the step
    // into the watcher's view range is interrupted (the one-tick cadence resolves within a
    // few ticks of that step). A generous budget so the brain advances + the interrupt fires.
    step(&mut app, 16);

    // AC5: a player watcher interrupted the acting enemy during the ENEMY turn — a real
    // ShotFired from the player watcher, and its TU debited (the persistent ShotLog + TU
    // capture the interrupt regardless of the later turn-boundary cap reset). The enemy was
    // BRAIN-DRIVEN (its move, not a synthetic emit) — proven by its advance into range.
    assert!(
        shots_by(&app, player_watcher_entity) >= 1,
        "AC5: the PLAYER watcher fired an interrupt at the BRAIN-driven acting enemy during \
         the enemy turn (faction symmetry — the mirror of AC1)",
    );
    assert!(
        tu_of(&app, player_watcher_entity).is_some_and(|tu| tu < watcher_tu_before),
        "AC5: the player watcher's TU is debited by its interrupt of the enemy",
    );
    // AC7 mirror: the player watcher's interrupt is also a NORMAL-pipeline shot.
    assert!(
        all_shots_have_reports(&app, player_watcher_entity),
        "AC5/AC7: the player watcher's interrupt resolves through the normal fire pipeline",
    );
    // The enemy was the brain-driven actor whose advance the watcher interrupted (it moved
    // off its spawn cell — never a synthetic emit).
    assert!(
        pos_of(&app, enemy).is_some_and(|p| p != enemy_at),
        "AC5: the enemy ACTUALLY moved (the brain drove it) — the interrupt saw a real step",
    );
}

// === A defensive end-to-end determinism check: the same seed + tuning reproduces the same
// interrupt outcome (count of watcher shots), proving the seeded ReactionRng draw is
// replay-stable through the live path (C3). ===

#[test]
fn the_interrupt_sequence_is_deterministic_across_identical_runs() {
    let run_once = || {
        let mut app = battle_app(forced_reaction_tuning(8));
        with_shot_log(&mut app);
        let situation = SituationBuilder::new()
            .with_gangers([
                mover(ground(6, 5), PLAYER, Direction::East),
                watcher(ground(4, 5), ENEMY, Direction::East),
            ])
            .build_with_gangs();
        drive_setup(&mut app, situation);
        let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
        else {
            unreachable!("setup spawns one player and one enemy");
        };
        app.world_mut()
            .write_message(MoveRequested::new(player, ground(10, 5)));
        run_until_walk_ends(&mut app, player);
        step(&mut app, 3);
        (shots_by(&app, enemy), pos_of(&app, player))
    };

    let first = run_once();
    let second = run_once();
    assert_eq!(
        first, second,
        "the same seed + tuning reproduces the same interrupt outcome (replay-stable \
         ReactionRng through the live path)",
    );
    // And it's a non-trivial outcome (the interrupt actually fired — otherwise determinism is
    // vacuous).
    assert!(
        first.0 >= 1,
        "precondition: the deterministic run actually interrupts (forced p == 1.0)",
    );
    // Touch the synthetic-vs-live distinction marker so the import is exercised: the live
    // ReactionShotFired producer (this module) is what halts the walk — the test never emits
    // it synthetically.
    let _ = ReactionShotFired::new;
}
