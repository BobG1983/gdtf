//! Per-field ganger state as **separate ECS components** — the E1.2 decomposition.
//!
//! A ganger is not one monolithic struct. Each piece of its battle state is its
//! own Bevy [`Component`] so a system can query **any subset** without touching
//! the others — change-detection, archetype filters, and disjoint queries all
//! work per field. The monolithic `GangerState` is deliberately **not** modelled
//! here (the GTW-6 architectural ruling); a system that only cares about `Hp`
//! queries `&Hp` alone, never a god-struct.
//!
//! Every value carries its meaning in its type (no-bare-types): a px-free grid
//! key is wrapped in [`Position`], a turn count in [`Tu`], and so on. The
//! newtypes use the E1.1 house style — a **private** inner field plus a derived
//! [`Deref`] (never a hand-written `impl Deref`) — and the inner direction /
//! stance / life kinds are **named domain enums**, not bare primitives.
//!
//! [`Default`] gives each component its documented **structural** initial value
//! (a fresh, unhurt, standing ganger: [`LifeState::Alive`], [`StanceKind::Standing`],
//! aim off, zero counts). These are invariants of "a newly-spawned ganger", not
//! tunable balance magnitudes. See `docs/combat/combat.md`, `resolution.md`,
//! `wounds-and-roster.md`, and `stats.md`.

use bevy::prelude::{Component, Deref};

use crate::metric::CellLevel;

/// A ganger's grid position — the `(cell, level)` key it occupies.
///
/// Wraps the E1.1 [`CellLevel`] (cell x/y + storey index) so a ganger's location
/// is the same `(cell, level)` identity the coarse occupancy and cover ledger are
/// keyed by (combat.md: "position everywhere is the pair `(cell, level)`"). A
/// distinct component from the rest of ganger state so movement systems can query
/// `&Position` / `&mut Position` alone.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position(CellLevel);

impl Position {
    /// Build a ganger position from the `(cell, level)` key it occupies.
    ///
    /// The one public constructor for the position component (private inner +
    /// constructor, the crate's newtype house style) — the move verbs and the
    /// occupancy-maintenance systems (E1.7) build a `Position` through this rather
    /// than reaching the private field.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self(at)
    }
}

/// One of the eight grid facings a ganger can turn to face.
///
/// The square grid's 8-way compass (cardinals + diagonals): a ganger turns in
/// place between these (combat.md's "turn in place" TU action), and the facing
/// drives the per-facing barrel offset and the faced cell tested for the bracing
/// bonus (resolution.md §1). Eight discrete directions, not a continuous angle —
/// the coarse model reasons in grid steps.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    /// Toward −Y.
    #[default]
    North,
    /// Toward +X, −Y.
    NorthEast,
    /// Toward +X.
    East,
    /// Toward +X, +Y.
    SouthEast,
    /// Toward +Y.
    South,
    /// Toward −X, +Y.
    SouthWest,
    /// Toward −X.
    West,
    /// Toward −X, −Y.
    NorthWest,
}

/// A ganger's facing — which of the eight grid [`Direction`]s it currently faces.
///
/// A distinct component so a turn/LOS system can query `&Facing` alone. Defaults
/// to [`Direction::North`] (a structural spawn default — the canonical "facing up
/// the grid" orientation, not a balance value).
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Facing(Direction);

impl Facing {
    /// Build a facing from the [`Direction`] the ganger faces.
    ///
    /// The public constructor (private inner + constructor, the crate's newtype
    /// house style) so the situation→entities setup (E1.8 / GTW-158) can build a
    /// `Facing` from an authored direction without reaching the private field.
    #[must_use]
    pub const fn new(direction: Direction) -> Self {
        Self(direction)
    }
}

/// The three postures a ganger can hold.
///
/// Posture reshapes the stability score and the clearance silhouette
/// (resolution.md §1: "prone 40 / kneel 25 / stand 10"; §4 stance reshapes the
/// bands). `Crouching` is the doc's "kneel" posture. Named domain kinds, not a
/// bare integer.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StanceKind {
    /// Upright — full stability gate, full silhouette (the doc's "stand").
    #[default]
    Standing,
    /// Kneeling — compressed silhouette, steadier than standing (the doc's "kneel").
    Crouching,
    /// Flat — lowest silhouette, steadiest, but cannot clear even LOW cover.
    Prone,
}

/// A ganger's stance — which [`StanceKind`] posture it currently holds.
///
/// A distinct component so a stability / clearance system can query `&Stance`
/// alone. Defaults to [`StanceKind::Standing`] (the structural spawn posture, not
/// a tunable).
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Stance(StanceKind);

impl Stance {
    /// Build a stance from the [`StanceKind`] posture the ganger holds.
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup can build
    /// a `Stance` from an authored posture without reaching the private field.
    #[must_use]
    pub const fn new(posture: StanceKind) -> Self {
        Self(posture)
    }
}

/// Whether a ganger is **aiming** (aimed shot) rather than hip-firing.
///
/// The Aim-Mode axis from resolution.md §1a: aiming narrows the dispersion cone
/// (×0.6) at a TU premium, hip-fired does not. A distinct component so a shot /
/// HUD system can query `&Aiming` alone. Defaults to `false` (hip-fired — a fresh
/// ganger is not aiming; a structural default, not a balance value).
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Aiming(bool);

impl Aiming {
    /// Build an aim-mode flag — `true` for aimed fire, `false` for hip-fired.
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup can build
    /// an `Aiming` from an authored value without reaching the private field.
    #[must_use]
    pub const fn new(aiming: bool) -> Self {
        Self(aiming)
    }
}

/// A gang (faction) identity — which side a ganger fights for.
///
/// Wraps a small gang index (glossary: a *Gang* is a faction / the player's
/// roster as a unit). The shooter/target faction decides friend from foe for
/// targeting and the friendly-fire path (resolution.md §2: "any other actor in
/// the path — including your own gang"). A distinct component so a targeting
/// system can query `&Faction` alone. Defaults to gang `0`.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Faction(u8);

impl Faction {
    /// Build a faction (gang) identity from its small gang index.
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup can build
    /// a `Faction` from an authored gang index without reaching the private field.
    #[must_use]
    pub const fn new(gang: u8) -> Self {
        Self(gang)
    }
}

/// A ganger's hit points — the in-battle raw-damage knock-down pool.
///
/// HP is the knock-down pool: damage depletes it and `HP ≤ 0` **downs** the
/// ganger (never kills directly — that is [`Wounds`]). A `u16` count
/// (stats.md "raw in-battle damage pool"). A distinct component so a damage
/// system can query `&mut Hp` alone. Defaults to `0`.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Hp(u16);

impl Hp {
    /// Build a hit-points pool from its count.
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup can build
    /// an `Hp` from an authored count without reaching the private field.
    #[must_use]
    pub const fn new(hp: u16) -> Self {
        Self(hp)
    }
}

/// A ganger's Wounds — the small **life** pool; `Wounds ≤ 0` → Dead.
///
/// Wounds is the life pool, "small, < a dozen" (stats.md): every hit can spend it
/// by injury severity, and emptying it is death — even at full [`Hp`]
/// (wounds-and-roster.md). A `u8` count (the pool is tiny). A distinct component
/// so the wound / bleed-out path can query `&mut Wounds` alone. Defaults to `0`.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Wounds(u8);

impl Wounds {
    /// Build a Wounds (life) pool from its count.
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup can build
    /// a `Wounds` from an authored count without reaching the private field.
    #[must_use]
    pub const fn new(wounds: u8) -> Self {
        Self(wounds)
    }
}

/// A ganger's Time Units — the per-turn action budget; unspent TU funds reactions.
///
/// Every action (step, turn, shot, kneel) spends from this pool, and leftover TU
/// fuels reaction fire on the enemy turn (combat.md / stats.md TU economy). A
/// `u8` budget. A distinct component so the action-economy system can query
/// `&mut Tu` alone. Defaults to `0`.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Tu(u8);

impl Tu {
    /// Build a Time-Unit budget from its count.
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup can build
    /// a `Tu` from an authored budget without reaching the private field.
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

/// A ganger's terminal life state — the two-pool outcome machine.
///
/// The combat/death system owns this (it is a state component, not a stat):
/// `HP ≤ 0` → [`Downed`](LifeState::Downed) (alive, incapacitated, dying),
/// `Wounds ≤ 0` → [`Dead`](LifeState::Dead) (Dead trumps Downed). It drives
/// occupancy updates in E1.7 (a corpse / downed body frees or holds its cell
/// differently). A standalone named enum component (wounds-and-roster.md state
/// machine). Defaults to [`Alive`](LifeState::Alive).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LifeState {
    /// Up and fighting — full agency.
    #[default]
    Alive,
    /// HP gone, Wounds remaining — incapacitated but alive (bleeding out unless
    /// stabilized; can be executed or recovered).
    Downed,
    /// Wounds gone — dead in battle (from stacked injuries or one Fatal hit).
    Dead,
}

#[cfg(test)]
mod tests {
    use bevy::ecs::world::World;

    use super::*;
    use crate::metric::{Cell, CellLevel, Level};

    // --- C14(a): per-field decomposition — each component spawns + queries
    // INDEPENDENTLY, with NO sibling component on the entity. Each test spawns an
    // entity carrying exactly one of the nine components and asserts a query for
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
        assert_independent(Position(CellLevel::new(Cell::new(3, 4), Level::new(1))));
    }

    #[test]
    fn facing_inserts_and_queries_independently() {
        assert_independent(Facing(Direction::SouthEast));
    }

    #[test]
    fn stance_inserts_and_queries_independently() {
        assert_independent(Stance(StanceKind::Prone));
    }

    #[test]
    fn aiming_inserts_and_queries_independently() {
        assert_independent(Aiming(true));
    }

    #[test]
    fn faction_inserts_and_queries_independently() {
        assert_independent(Faction(2));
    }

    #[test]
    fn hp_inserts_and_queries_independently() {
        assert_independent(Hp(42));
    }

    #[test]
    fn wounds_inserts_and_queries_independently() {
        assert_independent(Wounds(7));
    }

    #[test]
    fn tu_inserts_and_queries_independently() {
        assert_independent(Tu(60));
    }

    #[test]
    fn life_state_inserts_and_queries_independently() {
        assert_independent(LifeState::Downed);
    }

    /// A multi-component disjoint-query proof: spawn an entity with TWO of the
    /// nine, then query each ALONE and assert each retrieves its own value — a
    /// single-field query never needs (or sees) its sibling, which is the whole
    /// point of the decomposition (C2 / C14a). A bare `World` query of `&Hp` and
    /// a separate `&Tu` over the same entity proves the fields are addressable in
    /// isolation.
    #[test]
    fn sibling_components_are_independently_queryable() {
        let mut world = World::new();
        let id = world.spawn((Hp(10), Tu(25))).id();

        let mut hp_query = world.query::<&Hp>();
        assert_eq!(hp_query.get(&world, id), Ok(&Hp(10)));

        let mut tu_query = world.query::<&Tu>();
        assert_eq!(tu_query.get(&world, id), Ok(&Tu(25)));
    }

    // --- C14(b): default construction produces the documented initial value for
    // each component — the structural spawn invariants (a fresh, unhurt, standing
    // ganger). These are invariants, not tunable magnitudes, so pinning them is
    // required by the acceptance criteria.

    #[test]
    fn defaults_are_the_documented_initial_values() {
        // Position has no Default (no canonical spawn cell) — placement is the
        // caller's, so it is intentionally absent from this pin.
        assert_eq!(Facing::default(), Facing(Direction::North));
        assert_eq!(Direction::default(), Direction::North);
        assert_eq!(Stance::default(), Stance(StanceKind::Standing));
        assert_eq!(StanceKind::default(), StanceKind::Standing);
        assert_eq!(Aiming::default(), Aiming(false));
        assert_eq!(Faction::default(), Faction(0));
        assert_eq!(Hp::default(), Hp(0));
        assert_eq!(Wounds::default(), Wounds(0));
        assert_eq!(Tu::default(), Tu(0));
        assert_eq!(LifeState::default(), LifeState::Alive);
    }

    /// The newtypes' derived [`Deref`] reaches their inner value (C12 mandates a
    /// derived `Deref` on every newtype). Built from arbitrary literals so this
    /// pins the Deref mechanism + target type, not a default value.
    #[test]
    fn newtypes_deref_to_inner() {
        assert_eq!(*Facing(Direction::West), Direction::West);
        assert_eq!(*Stance(StanceKind::Crouching), StanceKind::Crouching);
        assert!(*Aiming(true));
        assert_eq!(*Faction(5), 5u8);
        assert_eq!(*Hp(123), 123u16);
        assert_eq!(*Wounds(9), 9u8);
        assert_eq!(*Tu(80), 80u8);
        // Position derefs to CellLevel (C3); compare the whole inner key.
        let key = CellLevel::new(Cell::new(1, 2), Level::new(3));
        assert_eq!(*Position(key), key);
    }
}
