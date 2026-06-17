//! The ganger's numeric pools & stats — the [`Hp`]/[`Wounds`] life pools, the
//! [`Tu`]/[`TuMax`] action economy, the [`Shooting`]/[`Toughness`]/[`Luck`]
//! combat attributes, and the ganger's [`GangerName`] identity.

use bevy::prelude::{Component, Deref};
use serde::Deserialize;

/// A ganger's **name** — its human-facing identity (e.g. "Alex Mercer").
///
/// The ganger's display identity: the status panel's identity line renders this
/// (GTW-285) in place of the placeholder cell location, so a selected ganger reads
/// by NAME, not by where it stands. Carried on the spawned entity as its own
/// queryable Component, never folded into another field. The combat sim's §1/§6
/// math NEVER reads it — a name is presentation identity, not a balance magnitude.
///
/// A name newtype over [`String`] (no-bare-types: a name is a domain value, not a
/// bare `String`), the [`WeaponName`](crate::weapon::WeaponName) precedent. Private
/// inner + derived [`Deref`] (house style — never a hand-written `impl Deref`).
/// `#[serde(transparent)]` lets an authored situation `.ron`'s `name` parse as a
/// bare string, so it deserializes as part of
/// [`GangerSpawn`](crate::situation::GangerSpawn) (the [`TuMax`] / [`WeaponName`]
/// serde-transparent shape). A `#[derive(Component)]` so the status panel can query
/// `&GangerName` off the selected entity.
#[derive(Deref, Component, Debug, Clone, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct GangerName(String);

impl GangerName {
    /// Build a ganger name from its display string.
    ///
    /// The public constructor (house style) so the E1.8 / GTW-158 setup and tests can
    /// build a `GangerName` without reaching the private field.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
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
pub struct Hp(pub(super) u16);

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
pub struct Wounds(pub(super) u8);

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
pub struct Tu(pub(super) u8);

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
/// `#[serde(transparent)]` lets an authored TU ceiling parse as a bare integer (the
/// sibling [`Tu`] shape), so an authored situation `.ron`'s `tu_max` deserializes as
/// part of [`GangerSpawn`](crate::situation::GangerSpawn).
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct TuMax(pub(super) u8);

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
pub struct Shooting(pub(super) f32);

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
pub struct Toughness(pub(super) f32);

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
pub struct Luck(pub(super) f32);

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
