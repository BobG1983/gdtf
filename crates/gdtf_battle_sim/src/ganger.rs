//! Per-field ganger state as **separate ECS components** — the E1.2 decomposition.
//!
//! A ganger is not one monolithic struct. Each piece of its battle state is its
//! own Bevy [`Component`] so a system can query **any subset** without touching
//! the others — change-detection, archetype filters, and disjoint queries all
//! work per field. The monolithic `GangerState` is deliberately **not** modelled
//! here (the GTW-6 architectural ruling); a system that only cares about `Hp`
//! queries `&Hp` alone, never a god-struct.
//!
//! Every value carries its meaning in its type (no-bare-types): a cubic-voxel
//! grid key is wrapped in [`Position`], a turn count in [`Tu`], and so on. The
//! newtypes use the E1.1 house style — a **private** inner field plus a derived
//! [`Deref`] (never a hand-written `impl Deref`) — and the inner direction /
//! stance / life kinds are **named domain enums**, not bare primitives.
//!
//! [`Default`] gives each component its documented **structural** initial value
//! (a fresh, unhurt, standing ganger: [`LifeState::Alive`], [`StanceKind::Standing`],
//! aim off, zero counts). These are invariants of "a newly-spawned ganger", not
//! tunable balance magnitudes. See `docs/combat/combat.md`, `resolution.md`,
//! `wounds-and-roster.md`, and `stats.md`.

use bevy::{
    math::Vec3,
    prelude::{Component, Deref},
};
use serde::Deserialize;

use crate::metric::{Cell, CellLevel};

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
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
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

impl Direction {
    /// This direction's **ground-plane unit step** in sim units — a normalised
    /// `Vec3` pointing the way the facing looks, with `z = 0` (the step lies in the
    /// ground plane; "up" is the separate `+z` axis, `Vec3::Z`).
    ///
    /// The components match the variant's documented orientation (the square grid's
    /// screen-free convention): North is −Y, East is +X, and each diagonal combines
    /// the two cardinals at equal magnitude. Every step is **unit length** — the
    /// cardinals are `±1` on one axis, the diagonals are `±1/√2` on each of two axes
    /// (so a diagonal is a unit vector, not a longer `(±1, ±1)`), making the per-facing
    /// forward offset a true sim-unit displacement (`docs/combat/battle-space.md`
    /// §"Sub-cell precision on the ground plane"). No pixel, no screen-coordinate
    /// convention — these are cubic-voxel sim units.
    #[must_use]
    pub fn forward_step(self) -> Vec3 {
        // The diagonal component: a unit vector's per-axis magnitude on the two
        // axes a diagonal spans (so √(d² + d²) = 1). Derived, not a pixel literal.
        let d = core::f32::consts::FRAC_1_SQRT_2;
        match self {
            Self::North => Vec3::new(0.0, -1.0, 0.0),
            Self::NorthEast => Vec3::new(d, -d, 0.0),
            Self::East => Vec3::new(1.0, 0.0, 0.0),
            Self::SouthEast => Vec3::new(d, d, 0.0),
            Self::South => Vec3::new(0.0, 1.0, 0.0),
            Self::SouthWest => Vec3::new(-d, d, 0.0),
            Self::West => Vec3::new(-1.0, 0.0, 0.0),
            Self::NorthWest => Vec3::new(-d, -d, 0.0),
        }
    }

    /// This direction's **ordinal** on the 8-way ring — `North = 0`, advancing
    /// clockwise through the compass to `NorthWest = 7`.
    ///
    /// The shared ring index the turn helpers ([`steps_to`](Self::steps_to),
    /// [`rotated_toward`](Self::rotated_toward)) reason in: a clockwise step is `+1`
    /// (mod 8), the short-way distance is computed from the ordinal gap. A loop
    /// index into the fixed eight-variant ring (the no-bare-types carve-out for an
    /// index into a collection you own), never a domain quantity stored on a
    /// component.
    const fn ordinal(self) -> u8 {
        match self {
            Self::North => 0,
            Self::NorthEast => 1,
            Self::East => 2,
            Self::SouthEast => 3,
            Self::South => 4,
            Self::SouthWest => 5,
            Self::West => 6,
            Self::NorthWest => 7,
        }
    }

    /// The [`Direction`] at ring ordinal `ord` (taken mod 8) — the inverse of
    /// [`ordinal`](Self::ordinal).
    ///
    /// Total: any `u8` maps to one of the eight variants by wrapping the ordinal
    /// into `0..8`, so the clockwise/counter-clockwise stepping in
    /// [`rotated_toward`](Self::rotated_toward) can never index out of the ring.
    const fn from_ordinal(ord: u8) -> Self {
        match ord % 8 {
            0 => Self::North,
            1 => Self::NorthEast,
            2 => Self::East,
            3 => Self::SouthEast,
            4 => Self::South,
            5 => Self::SouthWest,
            6 => Self::West,
            // 7 (and, after the mod, nothing else) is the only remaining ordinal.
            _ => Self::NorthWest,
        }
    }

    /// The **short-way** count of 45deg steps from this facing to `other` —
    /// `min(d, 8 - d)` where `d` is the ordinal gap. Range `0..=4`.
    ///
    /// `0` iff the two facings are equal, exactly `4` for an opposite facing
    /// (`North`↔`South`), and symmetric (`a.steps_to(b) == b.steps_to(a)`). This is
    /// the number of whole 45deg steps the [`crate::posture::set_facing`] verb must
    /// turn (and pay one [`crate::tuning::TurnTu`] for) to reach `other`. Returns a
    /// bare `u8` step *count* — a loop index / per-step multiplier into the
    /// `TurnTu` leaf (the no-bare-types carve-out for "indices into a collection you
    /// own"), NOT a domain quantity, and never stored on a component. Pure, total,
    /// no panic.
    #[must_use]
    pub const fn steps_to(self, other: Self) -> u8 {
        // The unsigned ordinal gap (both are in 0..8, so this never underflows).
        let d = self.ordinal().abs_diff(other.ordinal());
        // The short way around the ring of eight: never more than half (= 4).
        if d <= 8 - d { d } else { 8 - d }
    }

    /// The 8-way compass [`Direction`] pointing from cell `from` toward cell `to`,
    /// or `None` when the two cells coincide.
    ///
    /// Reads the sign of the `(to - from)` delta on each ground axis (`signum` of
    /// `dx` / `dy`): `dx == 0 && dy == 0` → `None` (no direction toward yourself);
    /// otherwise the sign pair picks one of the eight (e.g. `dx > 0, dy == 0` →
    /// `East`; `dx == 0, dy < 0` → `North` — smaller `y` is North, the `forward_step`
    /// −Y convention; `dx > 0, dy < 0` → `NorthEast`; `dx < 0, dy > 0` → `SouthWest`).
    /// The z axis is irrelevant to a ground facing, so a [`Cell`] (x/y only) is the
    /// input. Pure function of two cells, total, no panic.
    #[must_use]
    pub fn from_cells(from: Cell, to: Cell) -> Option<Self> {
        // signum collapses each axis delta to exactly -1 / 0 / +1 — the 8-way sign pair
        // (recall the −Y convention: smaller y is North). The full sign table is matched
        // on those three literals, so every pair is covered without ordered guards.
        let sx = (to.x - from.x).signum();
        let sy = (to.y - from.y).signum();
        let dir = match (sx, sy) {
            (0, 0) => return None, // coincident cells — no direction toward yourself
            (0, -1) => Self::North,
            (1, -1) => Self::NorthEast,
            (1, 0) => Self::East,
            (1, 1) => Self::SouthEast,
            (0, 1) => Self::South,
            (-1, 1) => Self::SouthWest,
            (-1, 0) => Self::West,
            // The last sign pair: (-1, -1).
            _ => Self::NorthWest,
        };
        Some(dir)
    }

    /// The facing reached after advancing up to `steps` 45deg steps the **short
    /// way** from this facing toward `target`, clamped so it never **overshoots**
    /// `target`.
    ///
    /// Short way: with `cw = (ord(target) + 8 - ord(self)) % 8` (clockwise gap) and
    /// `ccw = (ord(self) + 8 - ord(target)) % 8` (counter-clockwise gap), rotate
    /// clockwise (`+1` per step) when `cw <= ccw`, else counter-clockwise (`-1`), for
    /// `n = min(steps, min(cw, ccw))` steps. The opposite-facing tie (`cw == ccw ==
    /// 4`) breaks **clockwise**. So `d.rotated_toward(d, _)` and
    /// `d.rotated_toward(t, 0)` return `d` (`self`), and any `steps >=
    /// self.steps_to(target)` returns `target` (the clamp). This is what lets a
    /// PARTIAL turn land on an intermediate facing when the TU pool runs out before
    /// the full rotation. Pure, total, no panic.
    #[must_use]
    pub const fn rotated_toward(self, target: Self, steps: u8) -> Self {
        let from = self.ordinal();
        let to = target.ordinal();
        // The two ways around the ring (both in 0..8). cw + ccw == 8 unless equal.
        let cw = (to + 8 - from) % 8;
        let ccw = (from + 8 - to) % 8;
        // The short way's length; the tie (cw == ccw == 4) prefers clockwise.
        let short = if cw <= ccw { cw } else { ccw };
        // Clamp the requested steps so the turn never overshoots `target`.
        let n = if steps < short { steps } else { short };
        if cw <= ccw {
            Self::from_ordinal(from + n) // clockwise: +1 per step (mod 8 in from_ordinal)
        } else {
            // counter-clockwise: -1 per step, kept non-negative by adding a full ring.
            Self::from_ordinal(from + 8 - n)
        }
    }
}

/// A ganger's facing — which of the eight grid [`Direction`]s it currently faces.
///
/// A distinct component so a turn/LOS system can query `&Facing` alone. Defaults
/// to [`Direction::North`] (a structural spawn default — the canonical "facing up
/// the grid" orientation, not a balance value). `#[serde(transparent)]` lets an
/// authored facing parse as the bare [`Direction`] variant.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
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
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
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
/// a tunable). `#[serde(transparent)]` lets an authored stance parse as the bare
/// [`StanceKind`] variant.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
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
/// `#[serde(transparent)]` lets an authored aim-mode parse as a bare boolean.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
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
/// `#[serde(transparent)]` lets an authored gang index parse as a bare integer.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
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
/// system can query `&mut Hp` alone. Defaults to `0`. `#[serde(transparent)]`
/// lets an authored HP pool parse as a bare integer.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
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
/// `#[serde(transparent)]` lets an authored Wounds pool parse as a bare integer.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
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
/// `&mut Tu` alone. Defaults to `0`. `#[serde(transparent)]` lets an authored TU
/// budget parse as a bare integer.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
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

/// A ganger's **TU maximum** — the round-start Time-Unit budget the pool resets to.
///
/// The per-ganger ceiling that [`Tu`] is restored to at the start of each round
/// ([`crate::tu::reset_tu`]) — the round-start maximum, and the **denominator** of
/// GTW-38's reaction `TU_left / TU_max` ratio (resolution.md §8: `score = Reactions ×
/// (TU_left / TU_max)`). E4 only **defines** this max here; the reaction check that
/// reads the ratio is GTW-38.
///
/// A **distinct** component from [`Tu`] per no-bare-types rule 3 — same inner `u8`,
/// but a different concept (the ceiling, not the current pool), so the two are never
/// interchangeable. Private inner + derived [`Deref`], house style. A distinct
/// component so the action-economy / reaction path can query `&TuMax` alone. Defaults
/// to `0` (mirrors [`Tu`]'s structural spawn default — a fresh ganger carries no
/// budget until the situation setup authors one; not a tunable magnitude).
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TuMax(u8);

impl TuMax {
    /// Build a TU-maximum budget from its count.
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup can build a
    /// `TuMax` from an authored round-start budget without reaching the private field.
    #[must_use]
    pub const fn new(tu_max: u8) -> Self {
        Self(tu_max)
    }
}

/// A ganger's **Shooting** computed combat stat — the ranged-to-hit skill term.
///
/// The skill input to the §1b concentration exponent
/// `p = concentration_p(Shooting, weapon.accuracy)` (`docs/combat/stats.md`:
/// Shooting is "live today", derived `fn(Aim, Reflexes, Cool)`, and "feeds shot
/// concentration, `p = Shooting × weapon accuracy`"). Higher Shooting raises `p`,
/// clustering the in-cone draw toward dead-center.
///
/// Promoted from the E2.5 `sample_cone::Shooting` param-newtype to a queryable
/// ganger Component so a shot system can read the shooter's Shooting off the
/// entity (the source E4 cone composition + [`crate::sample_cone::concentration_p`]
/// both read this ONE type). A domain stat value (no-bare-types), dimensionless —
/// **zero pixels**. Private inner + derived [`Deref`]. The roster-side derivation
/// from attributes is campaign scope; this slice carries the value. A distinct
/// component so a shot system can query `&Shooting` alone. Defaults to `0.0`.
/// `#[serde(transparent)]` lets an authored Shooting stat parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Deserialize)]
#[serde(transparent)]
pub struct Shooting(f32);

impl Shooting {
    /// Build a Shooting value from its magnitude (dimensionless; higher = steadier
    /// aim → a larger concentration `p`).
    ///
    /// The public constructor (house style) so [`crate::sample_cone::concentration_p`]
    /// and the E1.8 / GTW-158 setup can build a `Shooting` without reaching the
    /// private field.
    #[must_use]
    pub const fn new(shooting: f32) -> Self {
        Self(shooting)
    }
}

/// A ganger's **Toughness** direct attribute — resistance to taking damage / being
/// wounded.
///
/// One of the eight core direct attributes (`docs/combat/stats.md`: "resistance to
/// taking damage / being wounded / Diseases / poisons"). In the severity roll it is
/// the defender's mitigation term — `−toughness_scale·Toughness` pushes the wound
/// score down (`docs/combat/wounds-and-roster.md` §"Rolling an injury"). This is the
/// per-ganger STAT carried on the entity, **distinct** from the tuning scalar
/// [`crate::tuning::ToughnessMitigation`] (the `k` coefficient that scales it). A
/// domain stat value (no-bare-types), dimensionless — **zero pixels**, a private
/// inner with a derived [`Deref`]. A distinct component so the severity path (E3.4)
/// can query `&Toughness` alone. Defaults to `0.0`. `#[serde(transparent)]` lets
/// an authored Toughness stat parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Deserialize)]
#[serde(transparent)]
pub struct Toughness(f32);

impl Toughness {
    /// Build a Toughness value from its magnitude (dimensionless; higher = harder to
    /// wound — a larger mitigation of the severity score).
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup can build a
    /// `Toughness` from an authored value without reaching the private field.
    #[must_use]
    pub const fn new(toughness: f32) -> Self {
        Self(toughness)
    }
}

/// A ganger's **Luck** direct attribute — directional fortune, shaping the severity
/// roll's one-sided random tail.
///
/// One of the eight core direct attributes (`docs/combat/stats.md`): a shooter's
/// Luck makes the wounds they deal nastier (adds to the severity score), a target's
/// Luck extends the low end of a hit's severity roll downward — a chance to shrug it
/// off (the floor moves, the ceiling is unchanged). It "feeds the severity roll only,
/// never the computed stats below". Both gangers' Luck stats are read in the severity
/// roll (E3.4 / E3.9): the shooter's via the tuning
/// [`crate::tuning::ShooterLuckScale`], the defender's via the
/// [`crate::tuning::DefenderLuckScale`]. A domain stat value (no-bare-types),
/// dimensionless — **zero pixels**. Private inner + derived [`Deref`]. A distinct
/// component so the severity path can query `&Luck` alone. Defaults to `0.0`.
/// `#[serde(transparent)]` lets an authored Luck stat parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Deserialize)]
#[serde(transparent)]
pub struct Luck(f32);

impl Luck {
    /// Build a Luck value from its magnitude (dimensionless; directional fortune —
    /// the shooter's adds to the severity score, the defender's shrinks its spread).
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup can build a
    /// `Luck` from an authored value without reaching the private field.
    #[must_use]
    pub const fn new(luck: f32) -> Self {
        Self(luck)
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
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
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

/// Whether a [`LifeState::Downed`] ganger has been **stabilized** — its bleed-out
/// clock halted.
///
/// The downed-ganger "stabilized, skip the bleed clock" flag
/// (`docs/combat/resolution.md` §9; `docs/combat/wounds-and-roster.md`
/// §"Downed → death … state machine"): an 8-adjacent ally's stabilize action
/// **sets** this `true`, after which the per-round bleed-out drain
/// ([`crate::bleed::tick_bleed`]) skips the ganger — no new *Bleeding Out* stacks,
/// the Wounds already drained stay drained, and the ganger **remains Downed**
/// (alive, out for the rest of the mission).
///
/// This slice (E3.7) is the flag's single **home**: [`tick_bleed`](crate::bleed::tick_bleed)
/// must **read** it to skip stabilized gangers, so it is defined here, not deferred
/// to the E3.8 stabilize action (which only **sets** this already-defined flag). A
/// distinct component so the bleed-out path can query `Option<&Stabilized>` alone.
/// Defaults to `false` (a freshly-downed ganger is **not** stabilized — the clock
/// runs until an ally dresses the wound; a structural spawn default, not a balance
/// value). Private inner + derived [`Deref`], house style.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Stabilized(bool);

impl Stabilized {
    /// Build a stabilized flag — `true` once an ally has dressed the downed
    /// ganger's wound (the clock halts), `false` while it still bleeds.
    ///
    /// The public constructor (private inner + constructor, the crate's newtype
    /// house style) so the E3.8 `stabilize_downed` action can set the flag, and the
    /// E3.7 bleed tests can spawn a stabilized ganger, without reaching the private
    /// field.
    #[must_use]
    pub const fn new(stabilized: bool) -> Self {
        Self(stabilized)
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::world::World;

    use super::*;
    use crate::metric::{Cell, CellLevel, Level};

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
    fn tu_max_inserts_and_queries_independently() {
        assert_independent(TuMax(60));
    }

    #[test]
    fn life_state_inserts_and_queries_independently() {
        assert_independent(LifeState::Downed);
    }

    #[test]
    fn stabilized_inserts_and_queries_independently() {
        assert_independent(Stabilized(true));
    }

    #[test]
    fn shooting_inserts_and_queries_independently() {
        assert_independent(Shooting(3.5));
    }

    #[test]
    fn toughness_inserts_and_queries_independently() {
        assert_independent(Toughness(4.0));
    }

    #[test]
    fn luck_inserts_and_queries_independently() {
        assert_independent(Luck(2.5));
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
        let shooter = world.spawn((Shooting(3.0), Toughness(2.0), Luck(1.0))).id();
        let defender = world.spawn((Shooting(1.0), Toughness(5.0), Luck(4.0))).id();

        // The severity-roll read shape: a tuple query over the three attribute stats.
        let mut q = world.query::<(&Shooting, &Toughness, &Luck)>();

        // The shooter contributes its Shooting (skill) + Luck (nastier wounds).
        let shooter_stats = q.get(&world, shooter);
        assert_eq!(
            shooter_stats,
            Ok((&Shooting(3.0), &Toughness(2.0), &Luck(1.0))),
            "the shooter's attribute stats are queryable off the entity",
        );

        // The defender contributes its Toughness (mitigation) + Luck (spread cap).
        let defender_stats = q.get(&world, defender);
        assert_eq!(
            defender_stats,
            Ok((&Shooting(1.0), &Toughness(5.0), &Luck(4.0))),
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
        // TuMax mirrors Tu's structural spawn default — a fresh ganger carries no
        // round-start budget until the situation setup authors one (not a tunable).
        assert_eq!(TuMax::default(), TuMax(0));
        assert_eq!(LifeState::default(), LifeState::Alive);
        // A freshly-downed ganger is NOT stabilized — the bleed clock runs until an
        // ally dresses the wound (a structural spawn default, not a tuning value).
        assert_eq!(Stabilized::default(), Stabilized(false));
        // The GTW-182 attribute stats default to 0.0 (a structural "no value yet"
        // spawn floor, not a tuning magnitude — real values are per-ganger data).
        assert_eq!(Shooting::default(), Shooting(0.0));
        assert_eq!(Toughness::default(), Toughness(0.0));
        assert_eq!(Luck::default(), Luck(0.0));
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
        // TuMax derefs to its inner u8 — the GTW-38 reaction ratio denominator.
        assert_eq!(*TuMax(120), 120u8);
        // Stabilized derefs to its inner bool (arbitrary value, mechanism not value).
        assert!(*Stabilized(true));
        // The GTW-182 attribute stats deref to their inner f32 (an f32 compare, so
        // an epsilon tolerance, not a bit-exact compare on a derived value).
        assert!((*Shooting(3.0) - 3.0).abs() < f32::EPSILON);
        assert!((*Toughness(4.5) - 4.5).abs() < f32::EPSILON);
        assert!((*Luck(2.0) - 2.0).abs() < f32::EPSILON);
        // Position derefs to CellLevel (C3); compare the whole inner key.
        let key = CellLevel::new(Cell::new(1, 2), Level::new(3));
        assert_eq!(*Position(key), key);
    }

    // --- GTW-168 AC #1: Direction::forward_step() — each of the 8 variants maps to
    // a documented ground-plane unit step in sim units, sign + axis matching the
    // named direction, and every step normalised (length 1 within f32 tolerance).
    // No pixel, no screen-coordinate convention.

    /// A loose f32 tolerance for the unit-length / component checks — the diagonal
    /// `1/√2` components are not exactly representable, so an exact compare is wrong.
    const STEP_TOL: f32 = 1.0e-6;

    #[test]
    fn forward_step_signs_and_axes_match_each_direction() {
        // The diagonal per-axis magnitude (positive); a diagonal spans two axes at
        // equal magnitude, the cardinals one axis at magnitude 1.
        let d = core::f32::consts::FRAC_1_SQRT_2;

        // (direction, expected x, expected y) — z is always 0 (ground plane).
        let cases = [
            (Direction::North, 0.0, -1.0),
            (Direction::NorthEast, d, -d),
            (Direction::East, 1.0, 0.0),
            (Direction::SouthEast, d, d),
            (Direction::South, 0.0, 1.0),
            (Direction::SouthWest, -d, d),
            (Direction::West, -1.0, 0.0),
            (Direction::NorthWest, -d, -d),
        ];

        for (dir, ex, ey) in cases {
            let step = dir.forward_step();
            assert!(
                (step.x - ex).abs() < STEP_TOL,
                "{dir:?}: x {} should match {ex}",
                step.x,
            );
            assert!(
                (step.y - ey).abs() < STEP_TOL,
                "{dir:?}: y {} should match {ey}",
                step.y,
            );
            // The step lies in the ground plane — z is exactly zero ("up" is the
            // separate +z axis).
            assert_eq!(step.z.to_bits(), 0.0_f32.to_bits(), "{dir:?}: z must be 0");
        }
    }

    #[test]
    fn forward_step_is_unit_length_for_every_direction() {
        for dir in [
            Direction::North,
            Direction::NorthEast,
            Direction::East,
            Direction::SouthEast,
            Direction::South,
            Direction::SouthWest,
            Direction::West,
            Direction::NorthWest,
        ] {
            let len = dir.forward_step().length();
            assert!(
                (len - 1.0).abs() < STEP_TOL,
                "{dir:?}: forward_step must be unit length, got {len}",
            );
        }
    }

    #[test]
    fn forward_step_diagonals_have_equal_axis_magnitude() {
        // A diagonal's two non-zero axes share one magnitude (so it points exactly
        // 45° between its two cardinals), distinguishing it from a longer (±1, ±1).
        for dir in [
            Direction::NorthEast,
            Direction::SouthEast,
            Direction::SouthWest,
            Direction::NorthWest,
        ] {
            let step = dir.forward_step();
            assert!(
                (step.x.abs() - step.y.abs()).abs() < STEP_TOL,
                "{dir:?}: diagonal axes must share magnitude: {} vs {}",
                step.x.abs(),
                step.y.abs(),
            );
        }
    }

    // --- GTW-235: the 8-way ring helpers. The fixed clockwise ring (the same
    // order the variants are declared in): North, NE, E, SE, S, SW, W, NW.
    const RING: [Direction; 8] = [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ];

    // GTW-235 AC1 — steps_to is the short-way 45deg count: for every ordered pair
    // a.steps_to(b) == min(d, 8-d) with d = |ord(a)-ord(b)|; 0 iff a == b; symmetric;
    // exactly 4 for every opposite pair; never exceeds 4.

    #[test]
    fn steps_to_is_the_short_way_45deg_count_for_every_pair() {
        for (ia, &a) in RING.iter().enumerate() {
            for (ib, &b) in RING.iter().enumerate() {
                // The ordinal gap, computed independently of the helper (the spec form).
                let ia = u8::try_from(ia).unwrap_or(0);
                let ib = u8::try_from(ib).unwrap_or(0);
                let d = ia.abs_diff(ib);
                let expected = d.min(8 - d);

                let got = a.steps_to(b);
                assert_eq!(got, expected, "{a:?}.steps_to({b:?}) must be min(d, 8-d)");
                // 0 iff equal.
                assert_eq!(got == 0, a == b, "{a:?}.steps_to({b:?}) is 0 iff equal");
                // Symmetric.
                assert_eq!(got, b.steps_to(a), "steps_to must be symmetric");
                // Never exceeds 4 (half the ring).
                assert!(got <= 4, "{a:?}.steps_to({b:?}) = {got} must not exceed 4");
            }
        }

        // Every opposite pair is exactly 4 (the four diameters of the ring).
        let opposites = [
            (Direction::North, Direction::South),
            (Direction::NorthEast, Direction::SouthWest),
            (Direction::East, Direction::West),
            (Direction::SouthEast, Direction::NorthWest),
        ];
        for (a, b) in opposites {
            assert_eq!(a.steps_to(b), 4, "{a:?} and {b:?} are opposite (4 steps)");
        }
    }

    // GTW-235 AC2 — from_cells gives the compass dir toward the target. Smaller y is
    // North (the forward_step −Y convention); coincident cells give None.

    #[test]
    fn from_cells_points_the_8_way_compass_toward_the_target() {
        let origin = Cell::new(5, 5);
        // The full 8-way table around (5,5): a neighbour in each compass direction.
        let cases = [
            (Cell::new(8, 5), Direction::East),      // dx>0, dy==0
            (Cell::new(5, 2), Direction::North),     // dx==0, dy<0 (smaller y is North)
            (Cell::new(8, 2), Direction::NorthEast), // dx>0, dy<0
            (Cell::new(2, 8), Direction::SouthWest), // dx<0, dy>0
            (Cell::new(2, 5), Direction::West),      // dx<0, dy==0
            (Cell::new(5, 8), Direction::South),     // dx==0, dy>0
            (Cell::new(8, 8), Direction::SouthEast), // dx>0, dy>0
            (Cell::new(2, 2), Direction::NorthWest), // dx<0, dy<0
        ];
        for (to, expected) in cases {
            assert_eq!(
                Direction::from_cells(origin, to),
                Some(expected),
                "from_cells({origin:?}, {to:?}) must point {expected:?}",
            );
        }

        // Coincident cells: no direction toward yourself.
        assert_eq!(
            Direction::from_cells(origin, origin),
            None,
            "from_cells of coincident cells must be None",
        );
        // Distance does not matter, only the sign pair (a far East cell is still East).
        assert_eq!(
            Direction::from_cells(Cell::new(0, 0), Cell::new(40, 0)),
            Some(Direction::East),
            "from_cells reads only the per-axis sign, not the magnitude",
        );
    }

    // GTW-235 AC3 — rotated_toward advances the short way, clamps (no overshoot), breaks
    // the opposite-facing tie clockwise, and is consistent with steps_to.

    #[test]
    fn rotated_toward_advances_the_short_way_clamped_with_clockwise_tie() {
        // Clockwise short way (North -> East is +2 clockwise).
        assert_eq!(
            Direction::North.rotated_toward(Direction::East, 1),
            Direction::NorthEast,
            "one step North toward East is NorthEast",
        );
        assert_eq!(
            Direction::North.rotated_toward(Direction::East, 2),
            Direction::East,
            "two steps North toward East reach East",
        );
        // Clamp: more steps than needed never overshoots the target.
        assert_eq!(
            Direction::North.rotated_toward(Direction::East, 9),
            Direction::East,
            "rotated_toward clamps — it never overshoots the target",
        );

        // Opposite facing (North <-> South, cw == ccw == 4): the tie breaks CLOCKWISE.
        assert_eq!(
            Direction::North.rotated_toward(Direction::South, 1),
            Direction::NorthEast,
            "the opposite-facing tie breaks clockwise (one step is NorthEast)",
        );
        assert_eq!(
            Direction::North.rotated_toward(Direction::South, 2),
            Direction::East,
            "two clockwise steps from North toward South reach East",
        );
        assert_eq!(
            Direction::North.rotated_toward(Direction::South, 4),
            Direction::South,
            "four steps from North reach the opposite South",
        );

        // Counter-clockwise short way (North -> West is -2 counter-clockwise).
        assert_eq!(
            Direction::North.rotated_toward(Direction::West, 1),
            Direction::NorthWest,
            "one step North toward West (the short way) is NorthWest",
        );

        // Identity: rotating toward self, or zero steps, stays put — for ALL facings.
        for &d in &RING {
            assert_eq!(d.rotated_toward(d, 3), d, "{d:?} toward itself stays put");
            for &t in &RING {
                assert_eq!(
                    d.rotated_toward(t, 0),
                    d,
                    "{d:?}.rotated_toward({t:?}, 0) must stay put",
                );
                // Consistency: rotating the full short-way count reaches the target.
                assert_eq!(
                    d.rotated_toward(t, d.steps_to(t)),
                    t,
                    "{d:?}.rotated_toward({t:?}, steps_to) must reach {t:?}",
                );
            }
        }
    }
}
