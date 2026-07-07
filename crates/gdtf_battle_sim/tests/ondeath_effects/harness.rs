//! Shared GTW-547 on-death fixture: the Explode weapon / barrel terrain / field
//! registries, the live battle-app driver, the combatant builders, and the damage
//! accessors — shared across BOTH effect families (explode + leave-field).

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

/// The single faction every fixture ganger belongs to (the `aoe_hittype` all-one-faction recipe — no
/// setup-time AI / reaction fire corrupts baselines; the blast striking teammates IS the
/// faction-blind friendly-fire property).
pub(crate) const PLAYER: u8 = 0;
/// A view range covering the whole cluster.
pub(crate) const TEST_VIEW_RANGE: u16 = 20;
/// A field def UUID for the on-death cover test (a `Cover` sim-kind carrying `on_death`).
pub(crate) const BARREL: TerrainUuid =
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0547_0547_0001));

/// A ground-floor `(cell, level)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A lethal, tight-cone single-fire weapon carrying an `Explode` on-death effect — a hit
/// connects straight down the axis (zero spread, `stable`, high accuracy) and a huge
/// damage/punch KILLS the struck ganger, whose death then fans the radius-1 blast.
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
        // GTW-547: the killed ganger detonates a radius-1 blast dealing a flat 50 HP per cell.
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

/// A [`WeaponRegistry`] whose `test-weapon` key (every setup-spawned ganger resolves it)
/// carries the Explode on-death weapon.
pub(crate) fn explode_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        explode_weapon_spec(),
    )])
}

/// A terrain registry holding the standard test defs PLUS a `BARREL` cover def carrying a
/// `LeaveField` on-death effect (the referenced field is seeded into the field catalog).
pub(crate) fn barrel_terrain_registry() -> TerrainDefRegistry {
    let mut base = gdtf_battle_sim::test_support::test_terrain_registry();
    base.insert(
        BARREL,
        TerrainDef {
            key:            BARREL,
            display_name:   TerrainDisplayName::new("Fuel Barrel".to_owned()),
            sim_kind:       TerrainSimKind::Cover {
                hp:               CoverHp::new(1), // 1 HP so one shot destroys it
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
        },
    );
    base
}

/// A field catalog holding the `burning` field the barrel leaves.
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

/// Build the live-runtime harness with `seed`, the Explode weapon, and (optionally) the barrel
/// terrain + field catalog for the `LeaveField` test.
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

/// Drive a setup through the REAL `setup_battle_on_request` Ok path with `seed` and settle it.
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

/// A high-Aim shooter at `at` facing `facing`.
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

/// A frail target at `at` — a shallow HP/Wounds pool so the huge Explode-weapon shot KILLS it
/// (flips it to Dead), triggering the on-death blast.
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

/// A sturdy bystander at `at` — a deep HP pool so it SURVIVES the on-death blast but visibly
/// LOSES HP to it (the "cells in radius take damage" assertion).
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

/// The ganger occupying `at` (the setup-published position), or `None`.
pub(crate) fn ganger_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Position)>();
    query
        .iter(world)
        .find(|(_, p)| ***p == at)
        .map(|(entity, _)| entity)
}

/// The current Hp of `entity`.
pub(crate) fn hp_of(app: &App, entity: Entity) -> Option<u16> {
    app.world().get::<Hp>(entity).map(|h| **h)
}

/// The current Wounds of `entity`.
pub(crate) fn wounds_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Wounds>(entity).map(|w| **w)
}

/// A ganger's `(Hp, Wounds)` snapshot.
pub(crate) fn vitals(app: &App, entity: Entity) -> (Option<u16>, Option<u8>) {
    (hp_of(app, entity), wounds_of(app, entity))
}

/// Whether `after` shows LESS Hp or LESS Wounds than `before` (both deplete under damage).
pub(crate) const fn took_damage(
    before: (Option<u16>, Option<u8>),
    after: (Option<u16>, Option<u8>),
) -> bool {
    matches!((before.0, after.0), (Some(b), Some(a)) if a < b)
        || matches!((before.1, after.1), (Some(b), Some(a)) if a < b)
}

/// Step `app` a fixed number of ticks so a written `FireRequested` dispatches + resolves +
/// `resolve_on_death` fans the effect.
pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

/// A LETHAL melee weapon spec — a huge `Swing` blow so a forced connect KILLS the target
/// outright (flips it to Dead), triggering the melee terminal-death gate. Arbitrary magnitudes
/// (mechanism only, not shipped tuning).
pub(crate) fn lethal_melee_spec() -> MeleeWeaponSpec {
    MeleeWeaponSpec {
        damage: WeaponDamage::new(500),
        punch: WeaponPunch::new(500),
        shred: WeaponShred::new(3),
        fatal_bias: FatalBias::new(50.0),
        ..test_melee_weapon_spec()
    }
}

/// A [`MeleeWeaponRegistry`] whose `fists` default (every fixture ganger resolves it, authoring
/// no melee weapon) is the LETHAL melee weapon — so a forced connect kills.
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

/// A strong melee attacker at `at` — high Fight contributors so a forced connect lands on the
/// defenceless victim (the `melee_act` `strong_attacker` recipe).
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

/// The current `LifeState` of `entity` (Alive default if absent — no `unwrap`).
pub(crate) fn life_of(app: &App, entity: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(entity)
        .copied()
        .unwrap_or(LifeState::Alive)
}
