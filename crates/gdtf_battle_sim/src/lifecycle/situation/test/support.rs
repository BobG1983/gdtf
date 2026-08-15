pub(super) use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::system::RunSystemOnce,
    prelude::{Commands, Entity, MinimalPlugins, World},
    scene::ScenePlugin,
};

pub(super) use super::super::*;
pub(super) use crate::{
    armor::{ArmorHardness, ArmorName, ArmorProtection, ArmorRegistry, ArmorSpec, BodyPart, Wears},
    cover::{CoverHp, CoverLedger, Destroyed, HeightBand},
    ganger::{
        Aiming, Facing, Faction, GangName, GangRegistry, GangRoster, GangerAttributes, GangerName,
        Hp, HpMax, LifeState, Luck, Position, Shooting, Stance, Toughness, Tu, TuMax, Wounds,
        WoundsMax, derive_stats,
    },
    inflicted_wound::InflictedWounds,
    metric::CellLevel,
    occupancy::{OccupancyGrid, TerrainKind},
    situation::CoverSpawn,
    surface::{SlabState, SurfaceGrid},
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainTag, TerrainUuid,
        },
        entity::{BlocksPathfinding, TerrainCell, TerrainPieceKind},
        facing::TerrainFacing,
        floor::FloorCostGrid,
        piece::{FootfallSound, TerrainGraphicKey},
    },
    test_support::{
        GangerSpawnBuilder, SituationBuilder, TEST_WEAPON_KEY, arbitrary_armor, ganger_at, key,
        test_armor_registry, test_gang_registry, test_melee_weapon_registry, test_terrain_registry,
        test_weapon_registry as test_registry,
    },
    tuning::GangerStatTuning,
    vertical::{InvalidVerticalLink, LinkKind, VerticalLink, VerticalLinkGraph},
    weapon::{FireMode, Weapon, WeaponName, WeaponRegistry, WeaponSpec, WieldedBy, Wields},
};

const SHIPPED_STUB_PISTOL_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/ranged/stub_pistol.weapon.ron"
));
const SHIPPED_LAS_CARBINE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/ranged/las_carbine.weapon.ron"
));
const SHIPPED_VOLATILE_CHARGE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/ranged/volatile_charge.weapon.ron"
));
const SHIPPED_GRENADE_LAUNCHER_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/ranged/grenade_launcher.weapon.ron"
));

pub(super) fn shipped_registry() -> Option<WeaponRegistry> {
    let stub_pistol = ron::de::from_str::<WeaponSpec>(SHIPPED_STUB_PISTOL_RON);
    let las_carbine = ron::de::from_str::<WeaponSpec>(SHIPPED_LAS_CARBINE_RON);
    let volatile_charge = ron::de::from_str::<WeaponSpec>(SHIPPED_VOLATILE_CHARGE_RON);
    let grenade_launcher = ron::de::from_str::<WeaponSpec>(SHIPPED_GRENADE_LAUNCHER_RON);
    assert!(
        stub_pistol.is_ok()
            && las_carbine.is_ok()
            && volatile_charge.is_ok()
            && grenade_launcher.is_ok(),
        "all shipped weapon files must parse: stub_pistol={stub_pistol:?} \
         las_carbine={las_carbine:?} volatile_charge={volatile_charge:?} \
         grenade_launcher={grenade_launcher:?}",
    );
    let (Ok(stub_pistol), Ok(las_carbine), Ok(volatile_charge), Ok(grenade_launcher)) =
        (stub_pistol, las_carbine, volatile_charge, grenade_launcher)
    else {
        return None;
    };
    Some(WeaponRegistry::new([
        (WeaponName::new("stub_pistol".to_owned()), stub_pistol),
        (WeaponName::new("las_carbine".to_owned()), las_carbine),
        (
            WeaponName::new("volatile_charge".to_owned()),
            volatile_charge,
        ),
        (
            WeaponName::new("grenade_launcher".to_owned()),
            grenade_launcher,
        ),
    ]))
}

const SHIPPED_FLAK_VEST_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/armor/flak_vest.armor.ron"
));
const SHIPPED_CARAPACE_PLATE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/armor/carapace_plate.armor.ron"
));

pub(super) fn shipped_armor_registry() -> Option<ArmorRegistry> {
    let flak_vest = ron::de::from_str::<ArmorSpec>(SHIPPED_FLAK_VEST_RON);
    let carapace_plate = ron::de::from_str::<ArmorSpec>(SHIPPED_CARAPACE_PLATE_RON);
    assert!(
        flak_vest.is_ok() && carapace_plate.is_ok(),
        "both shipped armor files must parse: flak_vest={flak_vest:?} carapace_plate={carapace_plate:?}",
    );
    let (Ok(flak_vest), Ok(carapace_plate)) = (flak_vest, carapace_plate) else {
        return None;
    };
    Some(ArmorRegistry::new([
        (ArmorName::new("flak_vest".to_owned()), flak_vest),
        (ArmorName::new("carapace_plate".to_owned()), carapace_plate),
    ]))
}

const SHIPPED_TOXIC_POOL_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/fields/toxic_waste_pool.field.ron"
));

pub(super) fn shipped_field_registry() -> Option<crate::effects::fields::FieldDefRegistry> {
    let toxic_pool = ron::de::from_str::<crate::effects::fields::FieldDef>(SHIPPED_TOXIC_POOL_RON);
    assert!(
        toxic_pool.is_ok(),
        "the shipped toxic_waste_pool field file must parse: {toxic_pool:?}",
    );
    let toxic_pool = toxic_pool.ok()?;
    Some(crate::effects::fields::FieldDefRegistry::new([(
        crate::effects::fields::FieldKey::new("toxic_waste_pool".to_owned()),
        toxic_pool,
    )]))
}

#[must_use]
pub(super) fn shipped_terrain_registry() -> TerrainDefRegistry {
    TerrainDefRegistry::default()
}

const SHIPPED_GANG_0_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/gangs/gang_0.gang.ron"
));
const SHIPPED_GANG_1_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/gangs/gang_1.gang.ron"
));

pub(super) fn shipped_gang_registry() -> Option<GangRegistry> {
    let gang_0 = ron::de::from_str::<GangRoster>(SHIPPED_GANG_0_RON);
    let gang_1 = ron::de::from_str::<GangRoster>(SHIPPED_GANG_1_RON);
    assert!(
        gang_0.is_ok() && gang_1.is_ok(),
        "both shipped gang files must parse: gang_0={gang_0:?} gang_1={gang_1:?}",
    );
    let (Ok(gang_0), Ok(gang_1)) = (gang_0, gang_1) else {
        return None;
    };
    Some(GangRegistry::new([
        (GangName::new("gang_0".to_owned()), gang_0),
        (GangName::new("gang_1".to_owned()), gang_1),
    ]))
}

pub(super) fn run_setup_with(
    situation: Situation,
    gangs: GangRegistry,
    registry: WeaponRegistry,
    armor: ArmorRegistry,
    terrain: Option<&TerrainDefRegistry>,
) -> Option<(App, BattleSetup)> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    let stat_tuning = GangerStatTuning::default();
    let melee = crate::test_support::test_melee_weapon_registry();
    let fallback_floor_cost = crate::tuning::CombatTuning::default().move_costs.open;
    let terrain_clone = terrain.cloned();
    let field_defs = shipped_field_registry().unwrap_or_default();
    let outcome = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                BattleRegistries::new(
                    &gangs,
                    &registry,
                    &melee,
                    &armor,
                    &stat_tuning,
                    terrain_clone.as_ref(),
                )
                .with_field_defs(&field_defs),
                fallback_floor_cost,
                &mut commands,
            )
        });
    assert!(outcome.is_ok(), "the one-shot setup system must run");
    let setup = outcome.ok().and_then(Result::ok);
    assert!(
        setup.is_some(),
        "setup_battle must succeed on a valid situation + registry",
    );
    let setup = setup?;
    app.update();
    Some((app, setup))
}

pub(super) fn single_def_registry(def: TerrainDef) -> TerrainDefRegistry {
    TerrainDefRegistry::new([(def.key, def)])
}

pub(super) fn attributes_of(placed: &crate::situation::PlacedGanger) -> GangerAttributes {
    let gangs = test_gang_registry();
    let Some(member) = gangs
        .roster(&placed.gang)
        .and_then(|roster| roster.member(&placed.member))
    else {
        return GangerAttributes {
            speed:     crate::ganger::Speed::new(0.0),
            aim:       crate::ganger::Aim::new(0.0),
            strength:  crate::ganger::Strength::new(0.0),
            toughness: Toughness::new(0.0),
            reflexes:  crate::ganger::Reflexes::new(0.0),
            cool:      crate::ganger::Cool::new(0.0),
            grit:      crate::ganger::Grit::new(0.0),
            luck:      Luck::new(0.0),
        };
    };
    member.attributes()
}

pub(super) fn minimal_fixture() -> (Situation, CellLevel, CellLevel, CellLevel, CellLevel) {
    let alice_at = key(5, 6, 0);
    let bob_at = key(7, 8, 0);
    let wall_cell = key(1, 2, 0);
    let slab_cell = key(3, 4, 1);

    let situation = SituationBuilder::new()
        .with_gangers([ganger_at(alice_at, 0), ganger_at(bob_at, 1)])
        .wall_at(wall_cell)
        .slab_at(slab_cell)
        .build();
    (situation, alice_at, bob_at, wall_cell, slab_cell)
}

pub(super) fn run_setup(situation: Situation) -> Option<(App, BattleSetup)> {
    let terrain = test_terrain_registry();
    run_setup_with(
        situation,
        test_gang_registry(),
        test_registry(),
        test_armor_registry(),
        Some(&terrain),
    )
}

pub(super) const SHIPPED_SITUATION_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/situations/skirmish.ron"
));

pub(super) fn shipped_situation() -> Option<Situation> {
    let parsed = ron::de::from_str::<Situation>(SHIPPED_SITUATION_RON);
    assert!(
        parsed.is_ok(),
        "shipped assets/content/situations/skirmish.ron must deserialize into Situation: {parsed:?}",
    );
    parsed.ok()
}
