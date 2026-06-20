//! Per-field decomposition / default / deref proofs for the ganger components.

use bevy::{ecs::world::World, prelude::Component};

use crate::{
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Position, Shooting, Stabilized,
        Stance, StanceKind, Toughness, Tu, TuMax, Wounds,
    },
    metric::{Cell, CellLevel, Level},
};

// --- C14(a): per-field decomposition — each component spawns + queries
// INDEPENDENTLY, with NO sibling component on the entity. Each test spawns an
// entity carrying exactly one of the ganger components and asserts a query for
// *only* that component finds it. This is the architectural proof that the
// state is decomposed per field: a single-component query compiles and works
// with no other component present. Using a bare `World` (no plugins) keeps it
// a true headless white-box test of the real ECS path.

/// Build a one-component entity in a fresh `World` and assert a query touching
/// **only** that component retrieves the expected value — proving the
/// component is independently insertable + queryable (C14a).
fn assert_independent<C: Component + Copy + PartialEq + core::fmt::Debug>(component: C) {
    let mut world = World::new();
    let id = world.spawn(component).id();
    // A query for ONLY this component — no sibling in the filter or the data.
    let mut query = world.query::<&C>();
    let got = query.get(&world, id);
    assert_eq!(
        got,
        Ok(&component),
        "single-component query must retrieve the lone component",
    );
    // And exactly one entity carries it — nothing else was implicitly spawned.
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
fn stabilized_inserts_and_queries_independently() {
    assert_independent(Stabilized::new(true));
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

// --- GTW-182 AC #1: a ganger constructed carrying Shooting/Toughness/Luck reads
// each attribute stat back through its derived Deref. The three are the attribute
// substrate the E3 severity roll (shooter Shooting, defender Toughness, both
// Luck) reads off the entity — magnitudes arbitrary (per-ganger data, not pinned).

/// A ganger carrying all three GTW-182 attribute stats reads each back via the
/// newtype's derived `Deref`, proving construct-and-read-back (AC #1). Distinct
/// arbitrary magnitudes so the readback is provably per-field, not a default.
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

/// GTW-182 AC #3: the three attribute stats are queryable off the entity. Spawn a
/// "shooter" and a "defender" entity each carrying all three, then a `Query`/
/// `World` access reads the shooter's Shooting/Luck and the defender's
/// Toughness/Luck — exactly the read shape the severity roll (E3.4 / E3.9)
/// performs. A bare `World` keeps it a true headless white-box test of the real
/// ECS path.
#[test]
fn shooter_and_defender_attribute_stats_are_queryable() {
    let mut world = World::new();
    let shooter = world
        .spawn((Shooting::new(3.0), Toughness::new(2.0), Luck::new(1.0)))
        .id();
    let defender = world
        .spawn((Shooting::new(1.0), Toughness::new(5.0), Luck::new(4.0)))
        .id();

    // The severity-roll read shape: a tuple query over the three attribute stats.
    let mut q = world.query::<(&Shooting, &Toughness, &Luck)>();

    // The shooter contributes its Shooting (skill) + Luck (nastier wounds).
    let shooter_stats = q.get(&world, shooter);
    assert_eq!(
        shooter_stats,
        Ok((&Shooting::new(3.0), &Toughness::new(2.0), &Luck::new(1.0))),
        "the shooter's attribute stats are queryable off the entity",
    );

    // The defender contributes its Toughness (mitigation) + Luck (spread cap).
    let defender_stats = q.get(&world, defender);
    assert_eq!(
        defender_stats,
        Ok((&Shooting::new(1.0), &Toughness::new(5.0), &Luck::new(4.0))),
        "the defender's attribute stats are queryable off the entity",
    );
}

/// A multi-component disjoint-query proof: spawn an entity with TWO of the
/// components, then query each ALONE and assert each retrieves its own value — a
/// single-field query never needs (or sees) its sibling, which is the whole
/// point of the decomposition (C2 / C14a). A bare `World` query of `&Hp` and
/// a separate `&Tu` over the same entity proves the fields are addressable in
/// isolation.
#[test]
fn sibling_components_are_independently_queryable() {
    let mut world = World::new();
    let id = world.spawn((Hp::new(10), Tu::new(25))).id();

    let mut hp_query = world.query::<&Hp>();
    assert_eq!(hp_query.get(&world, id), Ok(&Hp::new(10)));

    let mut tu_query = world.query::<&Tu>();
    assert_eq!(tu_query.get(&world, id), Ok(&Tu::new(25)));
}

// --- C14(b): default construction produces the documented initial value for
// each component — the structural spawn invariants (a fresh, unhurt, standing
// ganger). These are invariants, not tunable magnitudes, so pinning them is
// required by the acceptance criteria.

#[test]
fn defaults_are_the_documented_initial_values() {
    // Position has no Default (no canonical spawn cell) — placement is the
    // caller's, so it is intentionally absent from this pin.
    assert_eq!(Facing::default(), Facing::new(Direction::North));
    assert_eq!(Direction::default(), Direction::North);
    assert_eq!(Stance::default(), Stance::new(StanceKind::Standing));
    assert_eq!(StanceKind::default(), StanceKind::Standing);
    assert_eq!(Aiming::default(), Aiming::new(false));
    assert_eq!(Faction::default(), Faction::new(0));
    assert_eq!(Hp::default(), Hp::new(0));
    assert_eq!(Wounds::default(), Wounds::new(0));
    assert_eq!(Tu::default(), Tu::new(0));
    // TuMax mirrors Tu's structural spawn default — a fresh ganger carries no
    // round-start budget until the situation setup authors one (not a tunable).
    assert_eq!(TuMax::default(), TuMax::new(0));
    assert_eq!(LifeState::default(), LifeState::Alive);
    // A freshly-downed ganger is NOT stabilized — the bleed clock runs until an
    // ally dresses the wound (a structural spawn default, not a tuning value).
    assert_eq!(Stabilized::default(), Stabilized::new(false));
    // The GTW-182 attribute stats default to 0.0 (a structural "no value yet"
    // spawn floor, not a tuning magnitude — real values are per-ganger data).
    assert_eq!(Shooting::default(), Shooting::new(0.0));
    assert_eq!(Toughness::default(), Toughness::new(0.0));
    assert_eq!(Luck::default(), Luck::new(0.0));
}

/// The newtypes' derived [`Deref`](bevy::prelude::Deref) reaches their inner
/// value (C12 mandates a derived `Deref` on every newtype). Built from arbitrary
/// literals so this pins the Deref mechanism + target type, not a default value.
#[test]
fn newtypes_deref_to_inner() {
    assert_eq!(*Facing::new(Direction::West), Direction::West);
    assert_eq!(*Stance::new(StanceKind::Crouching), StanceKind::Crouching);
    assert!(*Aiming::new(true));
    assert_eq!(*Faction::new(5), 5u8);
    assert_eq!(*Hp::new(123), 123u16);
    assert_eq!(*Wounds::new(9), 9u8);
    assert_eq!(*Tu::new(80), 80u8);
    // TuMax derefs to its inner u8 — the GTW-38 reaction ratio denominator.
    assert_eq!(*TuMax::new(120), 120u8);
    // Stabilized derefs to its inner bool (arbitrary value, mechanism not value).
    assert!(*Stabilized::new(true));
    // The GTW-182 attribute stats deref to their inner f32 (an f32 compare, so
    // an epsilon tolerance, not a bit-exact compare on a derived value).
    assert!((*Shooting::new(3.0) - 3.0).abs() < f32::EPSILON);
    assert!((*Toughness::new(4.5) - 4.5).abs() < f32::EPSILON);
    assert!((*Luck::new(2.0) - 2.0).abs() < f32::EPSILON);
    // Position derefs to CellLevel (C3); compare the whole inner key.
    let key = CellLevel::new(Cell::new(1, 2), Level::new(3));
    assert_eq!(*Position::new(key), key);
}
