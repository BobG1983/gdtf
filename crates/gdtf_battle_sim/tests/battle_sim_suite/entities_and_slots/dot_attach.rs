//! HARNESS NOTE (the `aoe_hittype` idiom): the sim crate is the LOW crate, so it cannot dev-dep
use bevy::{
    app::App,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use cobalt_test_utils::unwatched_asset_plugin;
use gdtf_battle_sim::{
    acts::FireRequested,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{
        Aim, Aiming, Cool, Direction, Facing, GangRegistry, Grit, Speed, Strength, Toughness,
    },
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, Position, Stance, StanceKind},
    rng::BattleSeed,
    situation::{GangerSpawn, PlacedGanger, Situation},
    test_support::{
        GangerSpawnBuilder, SituationBuilder, TEST_WEAPON_KEY, dot_turns, single_mode,
        test_armor_registry, test_melee_weapon_registry, test_weapon_spec,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, DamageType, Dot, DotDamage, DotProfile, FatalBias, FireMode,
        FireModeSpec, Kickback, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Stable,
        WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

const PLAYER: u8 = 0;

const TEST_VIEW_RANGE: u16 = 20;

const DOT_PER_TURN: u16 = 5;
const DOT_TURNS: u8 = 3;

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

const fn dot_profile() -> DotProfile {
    DotProfile::new(
        DotDamage::new(DOT_PER_TURN),
        DamageType::Plasma,
        dot_turns(DOT_TURNS),
    )
}

fn dot_weapon_spec(dot: Option<DotProfile>) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.0),
        accuracy: Accuracy::new(5.0),
        kickback: Kickback::new(0.0),
        fatal_bias: FatalBias::new(0.0),
        damage: WeaponDamage::new(20),
        punch: WeaponPunch::new(30),
        damage_type: DamageType::Plasma,
        fire_mode: FireMode::new(vec![single_mode(0.2, 1)]),
        stable: Stable::new(true),
        dot,
        ..test_weapon_spec()
    }
}

fn soaked_dot_weapon_spec() -> WeaponSpec {
    WeaponSpec {
        damage: WeaponDamage::new(1),
        punch: WeaponPunch::new(0),
        shred: WeaponShred::new(0),
        ..dot_weapon_spec(Some(dot_profile()))
    }
}

fn dot_registry(dot: Option<DotProfile>) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        dot_weapon_spec(dot),
    )])
}

fn soaked_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        soaked_dot_weapon_spec(),
    )])
}

fn battle_app(seed: u64, dot: Option<DotProfile>) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(dot_registry(dot));
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    (app, seed)
}

fn soaked_battle_app(seed: u64) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(soaked_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    (app, seed)
}

fn drive_setup(
    app: &mut App,
    seed: u64,
    situation_and_gangs: (Situation, Vec<PlacedGanger>, GangRegistry),
) {
    let (situation, placements, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        placements,
        BattleSeed::new(seed),
    ));
    for _ in 0..4 {
        app.update();
    }
}

fn shooter(at: CellLevel, facing: Direction) -> GangerSpawn {
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

fn target(at: CellLevel) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(10.0))
        .grit(Grit::new(40.0))
        .cool(Cool::new(40.0))
        .toughness(Toughness::new(40.0))
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

fn dot_of(app: &App, entity: Entity) -> Option<Dot> {
    app.world().get::<Dot>(entity).copied()
}

fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

const fn fire_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

fn fire_one_shot(app: &mut App, seed: u64) -> (Entity, Entity) {
    let situation = SituationBuilder::new()
        .with_gangers([shooter(ground(5, 5), Direction::East), target(ground(8, 5))])
        .build_with_gangs();
    drive_setup(app, seed, situation);

    let (Some(shooter_e), Some(target_e)) =
        (ganger_at(app, ground(5, 5)), ganger_at(app, ground(8, 5)))
    else {
        unreachable!("setup spawns the shooter + target at distinct cells");
    };

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        fire_mode(),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(app, 3);
    (shooter_e, target_e)
}

#[test]
fn a_penetrating_dot_shot_attaches_a_dot() {
    let (mut app, seed) = battle_app(0x5544_0A0A, Some(dot_profile()));
    let (_shooter, target_e) = fire_one_shot(&mut app, seed);

    let attached = dot_of(&app, target_e);
    assert_eq!(
        attached,
        Some(Dot::from_profile(dot_profile())),
        "a penetrating hit from a DOT weapon attaches a Dot built from the weapon's profile \
         (PIN-DISCRIMINATING: fails if the attach path is unwired)",
    );
}

#[test]
fn a_non_dot_weapon_attaches_nothing() {
    let (mut app, seed) = battle_app(0x5544_0F0F, None);
    let (_shooter, target_e) = fire_one_shot(&mut app, seed);

    assert!(
        dot_of(&app, target_e).is_none(),
        "a weapon with no DotProfile attaches no Dot — the identity property (a penetrating \
         hit still wounds, but seeds no lingering burn)",
    );
}

#[test]
fn a_fully_soaked_dot_shot_attaches_no_dot() {
    let (mut app, seed) = soaked_battle_app(0x5544_0B0B);
    let (_shooter, target_e) = fire_one_shot(&mut app, seed);

    assert!(
        dot_of(&app, target_e).is_none(),
        "a fully-soaked hit (PenetratingDamage == 0) attaches NO Dot — even though HP may \
         still bruise (the DOT gate is penetration, not HP loss)",
    );
}

#[test]
fn a_second_penetrating_dot_hit_refreshes_not_stacks() {
    let (mut app, seed) = battle_app(0x5544_0E0E, Some(dot_profile()));

    let situation = SituationBuilder::new()
        .with_gangers([shooter(ground(5, 5), Direction::East), target(ground(8, 5))])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let (Some(shooter_e), Some(target_e)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 5)),
    ) else {
        unreachable!("setup spawns the shooter + target");
    };

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        fire_mode(),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);
    let first = dot_of(&app, target_e);
    assert_eq!(
        first.map(|d| d.remaining_turns.get()),
        Some(DOT_TURNS),
        "the first penetrating DOT hit attaches the full profile turn count",
    );

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        fire_mode(),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);
    let second = dot_of(&app, target_e);
    assert_eq!(
        second.map(|d| d.remaining_turns.get()),
        Some(DOT_TURNS),
        "refresh-not-stack: a second penetrating DOT hit RESETS the remaining turns to the \
         profile ({DOT_TURNS}), never the stacked {}",
        DOT_TURNS * 2,
    );
}
