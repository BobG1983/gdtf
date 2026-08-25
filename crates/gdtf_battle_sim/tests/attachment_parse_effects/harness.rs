use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    battle::{BattleSimPlugin, SetupBattleRequested},
    effects::attachments::AttachmentEffect,
    equipment::attachments::{
        AttachmentName, AttachmentRegistry, AttachmentSlot, AttachmentSpec, FittedAttachments,
        SlotCapacity, WeaponSlots,
    },
    ganger::{Cool, Direction, Facing, GangRegistry, Grit, Speed, Strength, Toughness},
    magazine::{Magazine, ReloadTu},
    prelude::{Cell, CellLevel, Faction, Stance, StanceKind},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, single_mode, test_armor_registry, test_melee_weapon_registry,
        test_terrain_registry, test_weapon_spec,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, FatalBias, FireMode, Kickback, MagazineSize, WeaponName, WeaponPunch,
        WeaponRegistry, WeaponSpec,
    },
};

pub(crate) const SEED: u64 = 0x0A77_AC48;
pub(crate) const PLAYER: u8 = 0;
pub(crate) const ENEMY: u8 = 1;
pub(crate) const TEST_VIEW_RANGE: u16 = 12;
pub(crate) const WEAPON_KEY: &str = "test-weapon";
pub(crate) const ATTACHMENT_KEY: &str = "test-attachment";

pub(crate) const fn level0() -> gdtf_battle_sim::metric::Level {
    gdtf_battle_sim::metric::Level::new(0)
}

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), level0())
}

pub(crate) fn ranged_spec(attachment_keys: Vec<AttachmentName>) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.05),
        accuracy: Accuracy::new(5.0),
        kickback: Kickback::new(0.0),
        fatal_bias: FatalBias::new(3.0),
        punch: WeaponPunch::new(10),
        magazine: Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)),
        fire_mode: FireMode::new(vec![single_mode(0.2, 1)]),
        slots: WeaponSlots::new(vec![(AttachmentSlot::Rail, SlotCapacity::new(1))]),
        attachments: FittedAttachments::new(attachment_keys),
        ..test_weapon_spec()
    }
}

pub(crate) fn ranged_registry(attachment_keys: Vec<AttachmentName>) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(WEAPON_KEY.to_owned()),
        ranged_spec(attachment_keys),
    )])
}

pub(crate) fn attachment_registry(effects: Vec<AttachmentEffect>) -> AttachmentRegistry {
    AttachmentRegistry::new([(
        AttachmentName::new(ATTACHMENT_KEY.to_owned()),
        AttachmentSpec {
            display_name: WeaponName::new("Test Attachment".to_owned()),
            slot: AttachmentSlot::Rail,
            effects,
        },
    )])
}

pub(crate) fn battle_app(effects: Vec<AttachmentEffect>) -> App {
    let keys = if effects.is_empty() {
        Vec::new()
    } else {
        vec![AttachmentName::new(ATTACHMENT_KEY.to_owned())]
    };
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(ranged_registry(keys));
    app.insert_resource(attachment_registry(effects));
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app.insert_resource(test_terrain_registry());
    app
}

pub(crate) fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    for _ in 0..8 {
        app.update();
    }
}

pub(crate) fn ganger_of(app: &mut App, faction: u8) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
}

pub(crate) fn weapon_entity_of(app: &mut App, ganger: Entity) -> Option<Entity> {
    use bevy::ecs::relationship::Relationship;
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &gdtf_battle_sim::weapon::WieldedBy, &Accuracy)>();
    query
        .iter(world)
        .find(|(_, wielded, _)| wielded.get() == ganger)
        .map(|(entity, ..)| entity)
}

pub(crate) fn player_at(at: CellLevel, dir: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(dir))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(12.0))
        .strength(Strength::new(12.0))
        .grit(Grit::new(12.0))
        .cool(Cool::new(12.0))
        .build()
}

pub(crate) fn enemy_at(at: CellLevel) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(ENEMY))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(12.0))
        .strength(Strength::new(12.0))
        .grit(Grit::new(12.0))
        .cool(Cool::new(12.0))
        .toughness(Toughness::new(12.0))
        .build()
}
