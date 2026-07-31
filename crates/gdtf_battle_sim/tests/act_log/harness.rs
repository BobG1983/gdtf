//! Shared fixture for the act-log suite: a live battle app on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` path, plus act-log readers.
//!
//! Modelled on the `reaction_trigger` suite's harness (each integration suite is its own
//! crate, so the fixture is rebuilt here rather than shared across suites).

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActEntry, ActLog, ActProvenance, ActSeq},
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Cool, Direction, Facing, GangRegistry, Grit, Reflexes, Speed, Toughness},
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, Position, Stance, StanceKind, Tu},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, test_armor_registry, test_melee_weapon_registry, test_weapon_registry,
    },
    tuning::{
        CombatTuning, ReactionCapBase, ReactionCapPerReactions, ReactionPMax, ReactionPMin,
        ReactionTuning, SuppressionRadius, SuppressionStabilityPenalty, ViewRange,
    },
};

/// An arbitrary (not shipped tuning) seed for the test battle's RNG streams.
pub(crate) const SEED: u64 = 0x4EAC_7104;

/// Gang `0` is the player; gang `1` is the enemy.
pub(crate) const PLAYER: u8 = 0;
/// The opposing gang.
pub(crate) const ENEMY: u8 = 1;

/// A view range that lights a local field a few cells out.
const TEST_VIEW_RANGE: u16 = 6;

/// A ground-floor `(cell, level)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A `ReactionTuning` that FORCES every opposed check to succeed and caps interrupts at
/// `cap` per turn — so a reaction test's geometry, not its dice, decides the outcome.
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
        suppression_radius:  SuppressionRadius::new(0),
        suppression_penalty: SuppressionStabilityPenalty::new(0.0),
    }
}

/// Build the full live-runtime harness with the given reaction tuning.
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
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path and settle it.
pub(crate) fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    for _ in 0..4 {
        app.update();
    }
}

/// The entity of the ganger currently standing at `at`.
pub(crate) fn ganger_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Position)>();
    query
        .iter(world)
        .find(|(_, position)| ***position == at)
        .map(|(entity, _)| entity)
}

/// Overwrite `entity`'s current TU pool.
pub(crate) fn set_tu(app: &mut App, entity: Entity, value: u8) {
    let Some(mut tu) = app.world_mut().get_mut::<Tu>(entity) else {
        unreachable!("the fixture ganger carries a Tu pool");
    };
    *tu = Tu::new(value);
}

/// A high-Reactions standing watcher with a fat HP pool, facing `facing`.
pub(crate) fn watcher(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
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

/// A mover with an ample TU pool and a fat HP pool (so an interrupt cannot down it
/// mid-walk and truncate the fixture).
pub(crate) fn tough_mover(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .reflexes(Reflexes::new(5.0))
        .cool(Cool::new(5.0))
        .grit(Grit::new(200.0))
        .toughness(Toughness::new(200.0))
        .build()
}

/// One act-log entry flattened into the facts an assertion cares about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoggedFact {
    /// The entry's sequence number.
    pub(crate) seq:        ActSeq,
    /// The acting entity.
    pub(crate) actor:      Entity,
    /// Why it happened.
    pub(crate) provenance: ActProvenance,
    /// A stable NAME for the deed's variant — enough to assert order and kind without
    /// comparing `f32`-bearing payloads.
    pub(crate) deed:       &'static str,
}

/// The stable variant name of a deed.
pub(crate) const fn deed_name(deed: &ActDeed) -> &'static str {
    // A wildcard-free match, so a NEW deed variant fails to compile here until it is given
    // a name — the same forcing shape the wire projection will use.
    match *deed {
        ActDeed::TurnBegan { .. } => "TurnBegan",
        ActDeed::PostureChanged { .. } => "PostureChanged",
        ActDeed::Stepped { .. } => "Stepped",
        ActDeed::MovedTo { .. } => "MovedTo",
        ActDeed::MoveRefused { .. } => "MoveRefused",
        ActDeed::Fired { .. } => "Fired",
        ActDeed::RoundResolved { .. } => "RoundResolved",
        ActDeed::Reloaded { .. } => "Reloaded",
        ActDeed::MagazineChanged { .. } => "MagazineChanged",
        ActDeed::Injured { .. } => "Injured",
        ActDeed::VitalsChanged { .. } => "VitalsChanged",
        ActDeed::Fell { .. } => "Fell",
        ActDeed::Struck { .. } => "Struck",
        ActDeed::DiedAt { .. } => "DiedAt",
        ActDeed::Suppressed { .. } => "Suppressed",
        ActDeed::ArmorBroke { .. } => "ArmorBroke",
        ActDeed::DotStarted { .. } => "DotStarted",
        ActDeed::FieldStarted { .. } => "FieldStarted",
        ActDeed::BleedStarted => "BleedStarted",
        ActDeed::Bled => "Bled",
        ActDeed::DotTicked { .. } => "DotTicked",
        ActDeed::FieldTicked { .. } => "FieldTicked",
        ActDeed::CoverSmashed { .. } => "CoverSmashed",
        ActDeed::MeleeLanded { .. } => "MeleeLanded",
        ActDeed::ThrowLanded { .. } => "ThrowLanded",
        ActDeed::LifeChanged { .. } => "LifeChanged",
    }
}

/// Every retained act-log entry, flattened.
pub(crate) fn logged(app: &App) -> Vec<LoggedFact> {
    app.world()
        .get_resource::<ActLog>()
        .map(|log| {
            log.since(ActSeq::START)
                .map(|entry| LoggedFact {
                    seq:        entry.seq(),
                    actor:      entry.actor(),
                    provenance: entry.provenance(),
                    deed:       deed_name(entry.deed()),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Every retained deed named `name`, cloned WHOLE.
///
/// [`LoggedFact`] deliberately flattens a deed down to its variant NAME so an order
/// assertion never has to compare `f32`-bearing payloads. That drops the payload, and the
/// payload is the part a consumer APPLIES — so a mapping assertion needs the deed itself.
pub(crate) fn deeds_of(app: &App, name: &str) -> Vec<ActDeed> {
    app.world()
        .get_resource::<ActLog>()
        .map(|log| {
            log.since(ActSeq::START)
                .map(ActEntry::deed)
                .filter(|deed| deed_name(deed) == name)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Every logged entry whose deed is named `name`.
pub(crate) fn logged_of(app: &App, name: &str) -> Vec<LoggedFact> {
    logged(app)
        .into_iter()
        .filter(|fact| fact.deed == name)
        .collect()
}
