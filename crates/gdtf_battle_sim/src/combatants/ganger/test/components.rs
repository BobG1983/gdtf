use bevy::{ecs::world::World, prelude::Component};

use crate::{
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Position, Shooting, Stance,
        StanceKind, Toughness, Tu, TuMax, Wounds,
    },
    metric::{Cell, CellLevel, Level},
};

fn assert_independent<C: Component + Copy + PartialEq + core::fmt::Debug>(component: C) {
    let mut world = World::new();
    let id = world.spawn(component).id();
    let mut query = world.query::<&C>();
    let got = query.get(&world, id);
    assert_eq!(
        got,
        Ok(&component),
        "single-component query must retrieve the lone component",
    );
    assert_eq!(query.iter(&world).count(), 1);
}

#[test]
fn position_inserts_and_queries_independently() {
    assert_independent(Position::new(CellLevel::new(
        Cell::new(3, 4),
        Level::new(1),
    )));
}

#[test]
fn facing_inserts_and_queries_independently() {
    assert_independent(Facing::new(Direction::SouthEast));
}

#[test]
fn stance_inserts_and_queries_independently() {
    assert_independent(Stance::new(StanceKind::Prone));
}

#[test]
fn aiming_inserts_and_queries_independently() {
    assert_independent(Aiming::new(true));
}

#[test]
fn faction_inserts_and_queries_independently() {
    assert_independent(Faction::new(2));
}

#[test]
fn hp_inserts_and_queries_independently() {
    assert_independent(Hp::new(42));
}

#[test]
fn wounds_inserts_and_queries_independently() {
    assert_independent(Wounds::new(7));
}

#[test]
fn tu_inserts_and_queries_independently() {
    assert_independent(Tu::new(60));
}

#[test]
fn tu_max_inserts_and_queries_independently() {
    assert_independent(TuMax::new(60));
}

#[test]
fn life_state_inserts_and_queries_independently() {
    assert_independent(LifeState::Downed);
}

#[test]
fn shooting_inserts_and_queries_independently() {
    assert_independent(Shooting::new(3.5));
}

#[test]
fn toughness_inserts_and_queries_independently() {
    assert_independent(Toughness::new(4.0));
}

#[test]
fn luck_inserts_and_queries_independently() {
    assert_independent(Luck::new(2.5));
}

#[test]
fn attribute_stats_construct_and_read_back_via_deref() {
    let shooting = Shooting::new(2.5);
    let toughness = Toughness::new(4.0);
    let luck = Luck::new(1.5);
    assert!(
        (*shooting - 2.5).abs() < f32::EPSILON,
        "Shooting derefs to inner"
    );
    assert!(
        (*toughness - 4.0).abs() < f32::EPSILON,
        "Toughness derefs to inner",
    );
    assert!((*luck - 1.5).abs() < f32::EPSILON, "Luck derefs to inner");
}

#[test]
fn shooter_and_defender_attribute_stats_are_queryable() {
    let mut world = World::new();
    let shooter = world
        .spawn((Shooting::new(3.0), Toughness::new(2.0), Luck::new(1.0)))
        .id();
    let defender = world
        .spawn((Shooting::new(1.0), Toughness::new(5.0), Luck::new(4.0)))
        .id();

    let mut q = world.query::<(&Shooting, &Toughness, &Luck)>();

    let shooter_stats = q.get(&world, shooter);
    assert_eq!(
        shooter_stats,
        Ok((&Shooting::new(3.0), &Toughness::new(2.0), &Luck::new(1.0))),
        "the shooter's attribute stats are queryable off the entity",
    );

    let defender_stats = q.get(&world, defender);
    assert_eq!(
        defender_stats,
        Ok((&Shooting::new(1.0), &Toughness::new(5.0), &Luck::new(4.0))),
        "the defender's attribute stats are queryable off the entity",
    );
}

#[test]
fn sibling_components_are_independently_queryable() {
    let mut world = World::new();
    let id = world.spawn((Hp::new(10), Tu::new(25))).id();

    let mut hp_query = world.query::<&Hp>();
    assert_eq!(hp_query.get(&world, id), Ok(&Hp::new(10)));

    let mut tu_query = world.query::<&Tu>();
    assert_eq!(tu_query.get(&world, id), Ok(&Tu::new(25)));
}

#[test]
fn defaults_are_the_documented_initial_values() {
    assert_eq!(Facing::default(), Facing::new(Direction::North));
    assert_eq!(Direction::default(), Direction::North);
    assert_eq!(Stance::default(), Stance::new(StanceKind::Standing));
    assert_eq!(StanceKind::default(), StanceKind::Standing);
    assert_eq!(Aiming::default(), Aiming::new(false));
    assert_eq!(Faction::default(), Faction::new(0));
    assert_eq!(Hp::default(), Hp::new(0));
    assert_eq!(Wounds::default(), Wounds::new(0));
    assert_eq!(Tu::default(), Tu::new(0));
    assert_eq!(TuMax::default(), TuMax::new(0));
    assert_eq!(LifeState::default(), LifeState::Alive);
    assert_eq!(Shooting::default(), Shooting::new(0.0));
    assert_eq!(Toughness::default(), Toughness::new(0.0));
    assert_eq!(Luck::default(), Luck::new(0.0));
}

#[test]
fn newtypes_deref_to_inner() {
    assert_eq!(*Facing::new(Direction::West), Direction::West);
    assert_eq!(*Stance::new(StanceKind::Crouching), StanceKind::Crouching);
    assert!(*Aiming::new(true));
    assert_eq!(*Faction::new(5), 5u8);
    assert_eq!(*Hp::new(123), 123u16);
    assert_eq!(*Wounds::new(9), 9u8);
    assert_eq!(*Tu::new(80), 80u8);
    assert_eq!(*TuMax::new(120), 120u8);
    assert!((*Shooting::new(3.0) - 3.0).abs() < f32::EPSILON);
    assert!((*Toughness::new(4.5) - 4.5).abs() < f32::EPSILON);
    assert!((*Luck::new(2.0) - 2.0).abs() < f32::EPSILON);
    let key = CellLevel::new(Cell::new(1, 2), Level::new(3));
    assert_eq!(*Position::new(key), key);
}
