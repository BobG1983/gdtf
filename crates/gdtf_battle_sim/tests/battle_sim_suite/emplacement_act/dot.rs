//! A mounted gun carries the DOT its spec authors, and a round from it seeds that DOT.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::FireRequested,
    ganger::{Aiming, Direction, Facing, Hp, LifeState, Toughness, TuMax},
    magazine::mode_tu_cost,
    metric::CellLevel,
    prelude::{Faction, Position, Stance, StanceKind},
    situation::GangerSpawn,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, TEST_MOUNTED_WEAPON_KEY, dot_turns, emplacement_at,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Dot, DotDamage, DotProfile, FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        WeaponName, WeaponRegistry, WeaponSpec,
    },
};

use super::harness::*;

/// The seed both cases drive.
const SEED: u64 = 0x5543_1244;

/// Ticks a request is given to reach its dispatcher and settle.
const SETTLE_TICKS: u32 = 3;

/// The faction the target fights for.
const ENEMY: u8 = 1;

/// Per-turn damage the mounted gun's authored DOT deals.
const DOT_PER_TURN: u16 = 5;

/// Turns that DOT burns for.
const DOT_TURNS: u8 = 3;

/// The profile the MOUNTED spec authors; the carried gun authors none.
const fn mount_dot() -> DotProfile {
    DotProfile::new(
        DotDamage::new(DOT_PER_TURN),
        MOUNT_DAMAGE_TYPE,
        dot_turns(DOT_TURNS),
    )
}

/// A registry whose MOUNTED spec alone authors a DOT, on a gun tight enough to land its round.
fn mounted_dot_registry() -> WeaponRegistry {
    WeaponRegistry::new([
        (
            WeaponName::new(OWN_KEY.to_owned()),
            gun_spec(OWN_DAMAGE_TYPE),
        ),
        (
            WeaponName::new(TEST_MOUNTED_WEAPON_KEY.to_owned()),
            WeaponSpec {
                dot: Some(mount_dot()),
                ..aimed_gun_spec(MOUNT_DAMAGE_TYPE)
            },
        ),
    ])
}

/// A ganger of the enemy faction, carrying the suite's own gun and no DOT of its own.
fn enemy_at(at: CellLevel, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(ENEMY))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .weapon(WeaponName::new(OWN_KEY.to_owned()))
        .toughness(Toughness::new(12.0))
        .build()
}

/// The one enemy ganger standing on `at`, or a failure naming the cell and the count found.
fn enemy_on(app: &mut App, at: CellLevel) -> Entity {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction, &Position)>();
    let found: Vec<Entity> = query
        .iter(world)
        .filter(|(_, faction, position)| ***faction == ENEMY && ***position == at)
        .map(|(entity, ..)| entity)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "exactly one enemy ganger must stand on {at:?}, found {}",
        found.len(),
    );
    let [entity] = found[..] else {
        unreachable!("the count above is one");
    };
    entity
}

/// The live DOT burning on an entity, if one was seeded.
fn dot_of(app: &App, entity: Entity) -> Option<Dot> {
    app.world().get::<Dot>(entity).copied()
}

/// The DOT profile a weapon entity was spawned with, if its spec authored one.
fn dot_profile_of(app: &App, weapon: Entity) -> Option<DotProfile> {
    app.world().get::<DotProfile>(weapon).copied()
}

/// What a ganger's hit points read right now.
fn hp_of(app: &App, entity: Entity) -> Option<u16> {
    app.world().get::<Hp>(entity).map(|hp| **hp)
}

/// The single mode both cases fire, and the one the affordability precondition prices.
const fn single_shot() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.3),
        ModeShots::new(1),
    )
}

/// What firing `mode` charges this actor, or nothing when the world holds no price for it.
fn cost_of(app: &App, actor: Entity, mode: &FireModeSpec) -> Option<u8> {
    let world = app.world();
    let tu_max = world.get::<TuMax>(actor)?;
    let aiming = world.get::<Aiming>(actor)?;
    let tuning = world.get_resource::<CombatTuning>()?;
    Some(*mode_tu_cost(mode, tu_max, aiming, tuning))
}

/// A gunner seated on a mount whose spec authors the DOT, plus whatever else the case needs.
fn a_gunner_on_a_dot_mount(extras: Vec<GangerSpawn>) -> (App, Entity, Entity) {
    let (mut app, seed) = battle_app(SEED);
    // Force the enter-provoked interrupt rather than leaving it to a roll.
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        reaction: forced_reaction_tuning(1),
        ..Default::default()
    });
    app.insert_resource(mounted_dot_registry());
    let situation = SituationBuilder::new()
        .with_gangers([player_at(west_entry(), Direction::East)])
        .with_gangers(extras)
        .with_scatter(emplacement_at(seat()))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, seat());
    let actor = player_on(&mut app, west_entry());
    mount(&mut app, actor, emplacement);
    assert!(
        wields_mount(&mut app, actor),
        "PRECONDITION: the actor must wield the mount, or a missing profile below is a mount \
         that never spawned rather than a profile the spawn dropped; it wields {}",
        wields_mount(&mut app, actor),
    );
    (app, actor, emplacement)
}

#[test]
fn a_mounted_gun_carries_the_dot_its_spec_authors() {
    let (app, _actor, emplacement) = a_gunner_on_a_dot_mount(Vec::new());

    let carried = mount_entity(&app, emplacement).and_then(|mount| dot_profile_of(&app, mount));
    assert_eq!(
        carried,
        Some(mount_dot()),
        "the mount carries the DOT its spec authors, the way a carried gun does; it carries \
         {carried:?}",
    );
}

#[test]
fn a_round_fired_from_the_mount_seeds_its_dot_on_the_target() {
    let (mut app, actor, _emplacement) =
        a_gunner_on_a_dot_mount(vec![enemy_at(east_entry(), Direction::West)]);
    let target = enemy_on(&mut app, east_entry());
    assert!(
        matches!(app.world().get::<LifeState>(actor), Some(LifeState::Alive)),
        "PRECONDITION: the enter exchange must leave the actor alive to take the shot under \
         test; it is {:?}",
        app.world().get::<LifeState>(actor),
    );
    assert_eq!(
        dot_of(&app, target),
        None,
        "PRECONDITION: the target carries no DOT before the shot, or the reader below answers \
         about the enter exchange rather than the round; it carries {:?}",
        dot_of(&app, target),
    );
    let before = hp_of(&app, target);

    let mode = single_shot();
    let Some(cost) = cost_of(&app, actor, &mode) else {
        unreachable!("the actor carries TuMax + Aiming and the app carries CombatTuning");
    };
    assert!(
        tu_of(&app, actor).is_some_and(|tu| tu >= cost),
        "PRECONDITION: after the enter exchange the actor's pool must still cover the {cost} TU \
         the mode under test charges; it holds {:?}",
        tu_of(&app, actor),
    );
    let (cell, level) = east_entry().split();
    app.world_mut()
        .write_message(FireRequested::new(actor, mode, cell, level));
    step(&mut app, SETTLE_TICKS);

    assert!(
        hp_of(&app, target) < before,
        "the round must land, or the missing DOT below is a shot that never hit; the target's \
         HP went from {before:?} to {:?}",
        hp_of(&app, target),
    );
    assert!(
        dot_of(&app, target).is_some(),
        "a penetrating round from the mount seeds the mount's authored DOT on the target; it \
         carries {:?}",
        dot_of(&app, target),
    );
}
