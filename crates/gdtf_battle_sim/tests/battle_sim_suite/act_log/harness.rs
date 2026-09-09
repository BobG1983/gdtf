use bevy::{
    app::App,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use cobalt_test_utils::unwatched_asset_plugin;
use gdtf_battle_sim::{
    act_log::{ActDeed, ActEntry, ActLog, ActProvenance, ActSeq, ActWitnesses},
    armor::{ArmorHardness, ArmorProtection},
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    ganger::{Cool, Direction, Facing, GangRegistry, Grit, Reflexes, Speed, Toughness, TuMax},
    magazine::{LoadedRounds, Magazine},
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
    visibility::SquadVisibility,
    weapon::{FireMode, FireModeSpec, MagazineSize, Wields},
};

pub(crate) const SEED: u64 = 0x4EAC_7104;

pub(crate) const PLAYER: u8 = 0;
pub(crate) const ENEMY: u8 = 1;

const TEST_VIEW_RANGE: u16 = 6;

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

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

pub(crate) fn battle_app(reaction: ReactionTuning) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
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

pub(crate) fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    for _ in 0..4 {
        app.update();
    }
}

pub(crate) fn ganger_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Position)>();
    query
        .iter(world)
        .find(|(_, position)| ***position == at)
        .map(|(entity, _)| entity)
}

pub(crate) fn set_tu(app: &mut App, entity: Entity, value: u8) {
    let Some(mut tu) = app.world_mut().get_mut::<Tu>(entity) else {
        unreachable!("the fixture ganger carries a Tu pool");
    };
    *tu = Tu::new(value);
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoggedFact {
    pub(crate) seq:        ActSeq,
    pub(crate) actor:      Entity,
    pub(crate) provenance: ActProvenance,
    pub(crate) deed:       &'static str,
    pub(crate) witnesses:  ActWitnesses,
}

pub(crate) const fn deed_name(deed: &ActDeed) -> &'static str {
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
        ActDeed::TerrainPieceSmashed { .. } => "TerrainPieceSmashed",
        ActDeed::MeleeLanded { .. } => "MeleeLanded",
        ActDeed::ThrowLanded { .. } => "ThrowLanded",
        ActDeed::EnteredView { .. } => "EnteredView",
        ActDeed::LifeChanged { .. } => "LifeChanged",
    }
}

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
                    witnesses:  entry.witnesses().clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

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

pub(crate) fn logged_of(app: &App, name: &str) -> Vec<LoggedFact> {
    logged(app)
        .into_iter()
        .filter(|fact| fact.deed == name)
        .collect()
}

/// The single-shot fire mode of the first gun a ganger wields.
pub(crate) fn single_mode_of(app: &mut App, shooter: Entity) -> FireModeSpec {
    use bevy::ecs::relationship::RelationshipTarget as _;

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

/// Fill every gun a ganger wields with `rounds`.
pub(crate) fn load_magazine(app: &mut App, shooter: Entity, rounds: u16) {
    use bevy::ecs::relationship::RelationshipTarget as _;

    let world = app.world_mut();
    let wielded: Vec<Entity> = {
        let Some(wields) = world.get::<Wields>(shooter) else {
            unreachable!("the shooter wields weapons at setup");
        };
        wields.iter().collect()
    };
    for entity in wielded {
        let Some(existing) = world.get::<Magazine>(entity).copied() else {
            continue;
        };
        let Some(mut magazine) = world.get_mut::<Magazine>(entity) else {
            continue;
        };
        *magazine = Magazine::new(
            LoadedRounds::new(rounds),
            MagazineSize::new(rounds),
            existing.reload_tu(),
        );
    }
}

/// Raise a ganger's time-unit ceiling, so a case can hand it more than it spawned with.
pub(crate) fn set_tu_max(app: &mut App, entity: Entity, value: u8) {
    let Some(mut tu_max) = app.world_mut().get_mut::<TuMax>(entity) else {
        unreachable!("the fixture ganger carries a TuMax ceiling");
    };
    *tu_max = TuMax::new(value);
}

/// Whether the player squad's fog is lighting a cell right now.
pub(crate) fn squad_sees(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<SquadVisibility>()
        .is_some_and(|fog| *fog.is_cell_visible(&at))
}

/// Move a ganger, which is what makes the squad's fog recompute.
pub(crate) fn stand_at(app: &mut App, entity: Entity, at: CellLevel) {
    if let Ok(mut row) = app.world_mut().get_entity_mut(entity) {
        row.insert(Position::new(at));
    }
}

/// Run frames so a fog change and the log pass that reads it have both happened.
pub(crate) fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// Put a destructible piece of the given kind in the cover ledger at a cell.
pub(crate) fn seed_cover(app: &mut App, at: CellLevel, max_hp: u32, kind: TerrainPieceKind) {
    let entry = CoverEntry::seeded(
        CoverHp::new(max_hp),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
        kind,
    );
    app.world_mut()
        .resource_mut::<CoverLedger>()
        .insert(at, entry);
}
