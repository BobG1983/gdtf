use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    acts::ThrowGrenadeRequested,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{
        Aim, Cool, Direction, Facing, GangRegistry, Grit, Hp, Speed, Strength, Toughness, Wounds,
    },
    magazine::{Magazine, ReloadTu},
    metric::CellLevel,
    prelude::{Faction, Position, Stance, StanceKind},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    surface::{SlabState, SurfaceGrid},
    test_support::{
        GangerSpawnBuilder, SituationBuilder, TEST_WEAPON_KEY, test_armor_registry,
        test_melee_weapon_registry, test_weapon_spec,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, BlastRadius, DamageType, FatalBias, FireMode, FireModeSpec, HitType,
        Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, TrajectoryStyle,
        WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry, WeaponSpec,
    },
};

use super::harness::*;


fn grenade_spec(radius: u8) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.2),
        accuracy: Accuracy::new(0.8),
        kickback: Kickback::new(0.0),
        fatal_bias: FatalBias::new(2.0),
        damage: WeaponDamage::new(20),
        punch: WeaponPunch::new(30),
        damage_type: DamageType::Blast,
        magazine: Magazine::loaded(MagazineSize::new(4), ReloadTu::new(18)),
        fire_mode: FireMode::new(vec![FireModeSpec::with_hit_type(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.35),
            ModeShots::new(1),
            HitType::Blast {
                radius: BlastRadius::new(radius),
            },
        )]),
        trajectory: TrajectoryStyle::Arc,
        ..test_weapon_spec()
    }
}

fn grenade_registry(radius: u8) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        grenade_spec(radius),
    )])
}

fn battle_app(seed: u64, radius: u8) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(grenade_registry(radius));
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    (app, seed)
}

fn drive_setup(app: &mut App, seed: u64, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(seed)));
    for _ in 0..4 {
        app.update();
    }
}

fn thrower(at: CellLevel, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .aim(Aim::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(12.0))
        .build()
}

fn target(at: CellLevel) -> GangerSpawn {
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

fn ganger_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Position)>();
    query
        .iter(world)
        .find(|(_, p)| ***p == at)
        .map(|(entity, _)| entity)
}

fn vitals(app: &App, entity: Entity) -> (Option<u16>, Option<u8>) {
    (
        app.world().get::<Hp>(entity).map(|h| **h),
        app.world().get::<Wounds>(entity).map(|w| **w),
    )
}

const fn took_damage(before: (Option<u16>, Option<u8>), after: (Option<u16>, Option<u8>)) -> bool {
    matches!((before.0, after.0), (Some(b), Some(a)) if a < b)
        || matches!((before.1, after.1), (Some(b), Some(a)) if a < b)
}

fn set_roof(app: &mut App, cells: &[(i32, i32)], state: SlabState) {
    let mut surface = app.world_mut().resource_mut::<SurfaceGrid>();
    for &(x, y) in cells {
        surface.set_slab(at_level(x, y, 1), state);
    }
}

fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

#[test]
fn a_grenade_lobbed_through_a_roof_hole_damages_the_room_occupants() {
    let (mut app, seed) = battle_app(0x5546_0A0A, 1);
    let situation = SituationBuilder::new()
        .with_gangers([
            thrower(at_level(5, 5, 1), Direction::East),
            target(ground(5, 7)),
            target(ground(6, 7)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    set_roof(
        &mut app,
        &[(5, 5), (5, 6), (5, 7), (6, 6), (6, 7)],
        SlabState::Destroyed,
    );

    let (Some(thrower_e), Some(occ_a), Some(occ_b)) = (
        ganger_at(&mut app, at_level(5, 5, 1)),
        ganger_at(&mut app, ground(5, 7)),
        ganger_at(&mut app, ground(6, 7)),
    ) else {
        unreachable!("setup spawns the thrower + two room occupants");
    };
    let (a0, b0) = (vitals(&app, occ_a), vitals(&app, occ_b));

    app.world_mut()
        .write_message(ThrowGrenadeRequested::new(thrower_e, ground(5, 7)));
    step(&mut app, 3);

    assert!(
        took_damage(a0, vitals(&app, occ_a)),
        "the direct room occupant (5,7) is damaged by the lobbed blast (before {a0:?}, after {:?})",
        vitals(&app, occ_a),
    );
    assert!(
        took_damage(b0, vitals(&app, occ_b)),
        "the adjacent room occupant (6,7) is caught by the radius-1 blast (before {b0:?}, after {:?})",
        vitals(&app, occ_b),
    );
}

#[test]
fn a_grenade_lobbed_at_an_intact_roof_is_blocked_and_spares_the_room() {
    let (mut app, seed) = battle_app(0x5546_0B0B, 1);
    let situation = SituationBuilder::new()
        .with_gangers([
            thrower(at_level(5, 5, 1), Direction::East),
            target(ground(5, 7)),
            target(ground(6, 7)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    set_roof(
        &mut app,
        &[(5, 5), (5, 6), (5, 7), (6, 6), (6, 7)],
        SlabState::Present,
    );

    let (Some(thrower_e), Some(occ_a), Some(occ_b)) = (
        ganger_at(&mut app, at_level(5, 5, 1)),
        ganger_at(&mut app, ground(5, 7)),
        ganger_at(&mut app, ground(6, 7)),
    ) else {
        unreachable!("setup spawns the thrower + two room occupants");
    };
    let (a0, b0) = (vitals(&app, occ_a), vitals(&app, occ_b));

    app.world_mut()
        .write_message(ThrowGrenadeRequested::new(thrower_e, ground(5, 7)));
    step(&mut app, 3);

    assert_eq!(
        vitals(&app, occ_a),
        a0,
        "an intact roof spares the room occupant (5,7) — the lob is blocked above it",
    );
    assert_eq!(
        vitals(&app, occ_b),
        b0,
        "an intact roof spares the room occupant (6,7)",
    );
}

#[test]
fn a_blind_throw_resolves_without_a_facing_or_los_gate() {
    let (mut app, seed) = battle_app(0x5546_0C0C, 1);
    let situation = SituationBuilder::new()
        .with_gangers([
            thrower(ground(5, 5), Direction::North),
            target(ground(9, 5)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(thrower_e), Some(victim)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(9, 5)),
    ) else {
        unreachable!("setup spawns the thrower + the target");
    };
    let v0 = vitals(&app, victim);

    app.world_mut()
        .write_message(ThrowGrenadeRequested::new(thrower_e, ground(9, 5)));
    step(&mut app, 3);

    assert!(
        took_damage(v0, vitals(&app, victim)),
        "a blind lob (thrower facing away) still resolves and damages the target — no facing / \
         LOS gate for an Arc weapon (before {v0:?}, after {:?})",
        vitals(&app, victim),
    );
}
