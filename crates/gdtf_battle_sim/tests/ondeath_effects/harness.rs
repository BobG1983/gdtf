use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::CoverHp,
    effects::{
        fields::{
            FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, ImmuneArmorTypes,
        },
        on_death::OnDeathEffect,
    },
    ganger::{
        Aim, Aiming, Cool, Direction, Facing, GangRegistry, Grit, Hp, Speed, Strength, Toughness,
        Wounds,
    },
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, LifeState, Position, Stance, StanceKind},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    terrain::def::{
        TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind,
        TerrainUuid,
    },
    test_support::{
        GangerSpawnBuilder, TEST_MELEE_WEAPON_KEY, TEST_WEAPON_KEY, field_turns, single_mode,
        test_armor_registry, test_melee_weapon_registry, test_melee_weapon_spec, test_weapon_spec,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, BlastRadius, DamageType, FatalBias, FireMode, HitType, Kickback,
        MeleeWeaponRegistry, MeleeWeaponSpec, Stable, WeaponDamage, WeaponName, WeaponPunch,
        WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

pub(crate) const PLAYER: u8 = 0;
pub(crate) const TEST_VIEW_RANGE: u16 = 20;
pub(crate) const BARREL: TerrainUuid =
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0547_0547_0001));
pub(crate) const FUEL_SLAB: TerrainUuid =
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0547_0547_0002));

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(crate) fn explode_weapon_spec() -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.0),
        accuracy: Accuracy::new(5.0),
        kickback: Kickback::new(0.0),
        fatal_bias: FatalBias::new(50.0),
        damage: WeaponDamage::new(500),
        punch: WeaponPunch::new(500),
        fire_mode: FireMode::new(vec![single_mode(0.2, 1)]),
        stable: Stable::new(true),
        on_death: Some(OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      gdtf_battle_sim::effects::on_death::ExplodeDamage::new(50),
            damage_type: DamageType::Blast,
        }),
        ..test_weapon_spec()
    }
}

pub(crate) fn explode_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        explode_weapon_spec(),
    )])
}

pub(crate) fn barrel_terrain_registry() -> TerrainDefRegistry {
    let mut base = gdtf_battle_sim::test_support::test_terrain_registry();
    base.insert(
        BARREL,
        TerrainDef {
            key:            BARREL,
            display_name:   TerrainDisplayName::new("Fuel Barrel".to_owned()),
            sim_kind:       TerrainSimKind::Cover {
                hp:               CoverHp::new(1),
                armor_protection: gdtf_battle_sim::armor::ArmorProtection::new(0),
                armor_hardness:   gdtf_battle_sim::armor::ArmorHardness::new(0),
                height_band:      gdtf_battle_sim::cover::HeightBand::Low,
            },
            presenter_kind: TerrainPresenterKind::Cover {
                graphic_name: gdtf_battle_sim::terrain::piece::TerrainGraphicKey::new(
                    "cover".to_owned(),
                ),
            },
            tags:           Vec::new(),
            on_death:       Some(OnDeathEffect::LeaveField {
                field: FieldKey::new("burning".to_owned()),
            }),

            blocks_pathing: None,
            blocks_los:     None,
        },
    );
    base.insert(FUEL_SLAB, fuel_slab_def());
    base
}

/// A slab def carrying an authored `on_death`, so a destroyed slab can fan an effect.
fn fuel_slab_def() -> TerrainDef {
    TerrainDef {
        key:            FUEL_SLAB,
        display_name:   TerrainDisplayName::new("Fuel Slab".to_owned()),
        sim_kind:       TerrainSimKind::Slab {
            hp:               gdtf_battle_sim::slab::SlabHp::new(120),
            armor_protection: gdtf_battle_sim::armor::ArmorProtection::new(4),
            armor_hardness:   gdtf_battle_sim::armor::ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: gdtf_battle_sim::terrain::piece::TerrainGraphicKey::new(
                "slab".to_owned(),
            ),
            footfall:     None,
        },
        tags:           Vec::new(),
        on_death:       Some(OnDeathEffect::LeaveField {
            field: FieldKey::new("burning".to_owned()),
        }),
        blocks_pathing: None,
        blocks_los:     None,
    }
}

pub(crate) fn burning_field_registry() -> FieldDefRegistry {
    FieldDefRegistry::new([(
        FieldKey::new("burning".to_owned()),
        FieldDef::new(
            FieldDamage::new(3),
            DamageType::Plasma,
            ImmuneArmorTypes::default(),
            FieldDuration::Turns(field_turns(2)),
        ),
    )])
}

pub(crate) fn battle_app(seed: u64, with_barrel: bool) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(explode_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    if with_barrel {
        app.insert_resource(barrel_terrain_registry());
        app.insert_resource(burning_field_registry());
    }
    (app, seed)
}

pub(crate) fn drive_setup(
    app: &mut App,
    seed: u64,
    situation_and_gangs: (Situation, GangRegistry),
) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(seed)));
    for _ in 0..4 {
        app.update();
    }
}

pub(crate) fn shooter(at: CellLevel, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .aiming(Aiming::new(true))
        .speed(Speed::new(20.0))
        .aim(Aim::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(12.0))
        .build()
}

pub(crate) fn frail_target(at: CellLevel) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(10.0))
        .grit(Grit::new(1.0))
        .cool(Cool::new(1.0))
        .toughness(Toughness::new(1.0))
        .build()
}

pub(crate) fn bystander(at: CellLevel) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(10.0))
        .grit(Grit::new(30.0))
        .cool(Cool::new(30.0))
        .toughness(Toughness::new(30.0))
        .build()
}

pub(crate) fn ganger_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Position)>();
    query
        .iter(world)
        .find(|(_, p)| ***p == at)
        .map(|(entity, _)| entity)
}

pub(crate) fn hp_of(app: &App, entity: Entity) -> Option<u16> {
    app.world().get::<Hp>(entity).map(|h| **h)
}

pub(crate) fn wounds_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Wounds>(entity).map(|w| **w)
}

pub(crate) fn vitals(app: &App, entity: Entity) -> (Option<u16>, Option<u8>) {
    (hp_of(app, entity), wounds_of(app, entity))
}

pub(crate) const fn took_damage(
    before: (Option<u16>, Option<u8>),
    after: (Option<u16>, Option<u8>),
) -> bool {
    matches!((before.0, after.0), (Some(b), Some(a)) if a < b)
        || matches!((before.1, after.1), (Some(b), Some(a)) if a < b)
}

pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

pub(crate) fn lethal_melee_spec() -> MeleeWeaponSpec {
    MeleeWeaponSpec {
        damage: WeaponDamage::new(500),
        punch: WeaponPunch::new(500),
        shred: WeaponShred::new(3),
        fatal_bias: FatalBias::new(50.0),
        ..test_melee_weapon_spec()
    }
}

pub(crate) fn lethal_melee_registry() -> MeleeWeaponRegistry {
    MeleeWeaponRegistry::new([
        (
            WeaponName::new(gdtf_battle_sim::weapon::FISTS_KEY.to_owned()),
            lethal_melee_spec(),
        ),
        (
            WeaponName::new(TEST_MELEE_WEAPON_KEY.to_owned()),
            lethal_melee_spec(),
        ),
    ])
}

pub(crate) fn melee_attacker(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
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

pub(crate) fn life_of(app: &App, entity: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(entity)
        .copied()
        .unwrap_or(LifeState::Alive)
}
