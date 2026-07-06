//! Shared GTW-468 reaction fixture: the forced-reaction tuning, the live battle-app
//! driver, the `ShotFired` recorder, the actor builders, and the turn-cycling drivers.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins, Resource},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    acts::{EndTurnRequested, movement::WalkInProgress},
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Cool, Direction, Facing, GangRegistry, Reflexes, Speed},
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, Position, Stance, StanceKind, Tu},
    rng::BattleSeed,
    shot_fired::ShotFired,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, test_armor_registry, test_melee_weapon_registry, test_weapon_registry,
    },
    tuning::{
        CombatTuning, ReactionCapBase, ReactionCapPerReactions, ReactionPMax, ReactionPMin,
        ReactionTuning, ReactionsUsed, SuppressionRadius, SuppressionStabilityPenalty, ViewRange,
    },
};

/// An arbitrary (not shipped tuning) seed for the test battle's RNG streams.
pub(crate) const SEED: u64 = 0x4EAC_7104;

/// Gang `0` is the player; gang `1` is the enemy.
pub(crate) const PLAYER: u8 = 0;
pub(crate) const ENEMY: u8 = 1;

/// A view range that lights a local field a few cells out, yet is short enough that a cell
/// several tiles away is UNSEEN (so AC3's out-of-LOS-by-range case is constructible).
/// Arbitrary test tuning, never a pinned shipped magnitude.
pub(crate) const TEST_VIEW_RANGE: u16 = 6;

/// A ground-floor `(cell, level)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A `ReactionTuning` that FORCES every opposed check to succeed (`p_min == p_max == 1.0`)
/// and caps interrupts at exactly `cap` per turn (a flat base, zero per-`Reactions` slope).
#[expect(
    clippy::cast_precision_loss,
    reason = "the test caps are tiny (1 or 8), so the u32 -> f32 conversion is exact"
)]
pub(crate) const fn forced_reaction_tuning(cap: u32) -> ReactionTuning {
    ReactionTuning {
        cap_base:            ReactionCapBase::new(cap as f32),
        cap_per_reactions:   ReactionCapPerReactions::new(0.0),
        p_min:               ReactionPMin::new(1.0),
        p_max:               ReactionPMax::new(1.0),
        // GTW-526: the reaction tests are agnostic to suppression — radius 0 keeps this
        // fixture's suppression to the directly-targeted cell only (irrelevant to the
        // TU-debit / shot-fired / walk-halt assertions here). The stability penalty is
        // likewise irrelevant here (no shot-cone read in these assertions).
        suppression_radius:  SuppressionRadius::new(0),
        suppression_penalty: SuppressionStabilityPenalty::new(0.0),
    }
}

/// Build the FULL live-runtime harness (the `committed_walk` `battle_app` idiom) with a `CombatTuning`
/// carrying the given short view range + the given reaction tuning, plus the persistent
/// `Load` weapon/armor registries a `MinimalPlugins` app has no `AssetServer` to load.
pub(crate) fn battle_app(reaction: ReactionTuning) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        reaction,
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee
    // weapon resolves at setup (fixture gangers author none -> `fists`).
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path and settle it (the
/// deferred `bsn!` ganger scenes materialize and the first-run recompute fills the fog).
pub(crate) fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
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
pub(crate) fn ganger_of(app: &mut App, faction: u8) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
}

/// The current TU of `entity`.
pub(crate) fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

/// The current `(cell, level)` of `entity`.
pub(crate) fn pos_of(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(entity).map(|p| **p)
}

/// The current per-turn interrupt count of `entity`.
pub(crate) fn used_of(app: &App, entity: Entity) -> Option<u32> {
    app.world().get::<ReactionsUsed>(entity).map(|u| **u)
}

/// Whether `entity` still carries a `WalkInProgress` (the walk is in flight).
pub(crate) fn is_walking(app: &App, entity: Entity) -> bool {
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
pub(crate) struct ShotLog {
    /// One entry per `ShotFired` round emitted: the shooter, and whether it carried a
    /// `HitReport` (AC7 — a normal-pipeline shot carries one).
    rounds: Vec<(Entity, bool)>,
}

/// Drain `ShotFired` into the `ShotLog` recorder — added to the test app so the run's full
/// fire history is queryable after `app.update()`s.
pub(crate) fn record_shots(
    mut shots: bevy::prelude::MessageReader<ShotFired>,
    mut log: bevy::prelude::ResMut<ShotLog>,
) {
    for shot in shots.read() {
        log.rounds.push((shot.shooter, shot.report.is_some()));
    }
}

/// Add the `ShotFired` recorder to the app (after `BattleSimPlugin`, so the buffer exists).
pub(crate) fn with_shot_log(app: &mut App) {
    app.init_resource::<ShotLog>();
    app.add_systems(bevy::app::Update, record_shots);
}

/// How many rounds `shooter` fired across the run.
pub(crate) fn shots_by(app: &App, shooter: Entity) -> usize {
    app.world().get_resource::<ShotLog>().map_or(0, |log| {
        log.rounds.iter().filter(|(s, _)| *s == shooter).count()
    })
}

/// Whether every round `shooter` fired carried a `HitReport` (the normal pipeline; AC7).
pub(crate) fn all_shots_have_reports(app: &App, shooter: Entity) -> bool {
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
pub(crate) fn watcher(
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
pub(crate) fn mover(
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
        .build()
}

/// A mover/actor with an ample TU pool AND a FAT HP/Wounds pool (high Grit / Toughness /
/// Cool) — so a multi-turn test that repeatedly interrupt-shoots it cannot down it before
/// the assertions complete. Used for the AC4 actor (the enemy the watcher interrupts across
/// two turns).
pub(crate) fn tough_mover(
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
pub(crate) fn run_until_walk_ends(app: &mut App, entity: Entity) {
    for _ in 0..48 {
        app.update();
        if !is_walking(app, entity) {
            return;
        }
    }
}

/// Step `app` a fixed number of ticks (for a settle that is not gated on a walk ending).
pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

/// Whether the active faction is currently the player's.
pub(crate) fn player_turn_active(app: &App) -> bool {
    app.world()
        .get_resource::<gdtf_battle_sim::turn::ActiveFaction>()
        .is_some_and(|active| ***active == PLAYER)
}

/// Cycle turns (writing `EndTurnRequested` and letting the brain run / hand control back)
/// until it is the PLAYER's turn again, crossing at least one boundary — so the player
/// gangers' TU regenerate at the player turn-start AND every watcher's per-turn cap counter
/// is reset (`reset_reactions_used` on each `TurnStarted`). Bounded — never an open loop.
pub(crate) fn cycle_back_to_player_turn(app: &mut App) {
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
