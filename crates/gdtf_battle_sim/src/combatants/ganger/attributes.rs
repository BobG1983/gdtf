//! The eight **direct attributes** — the ganger's raw, slowly-changing potential
//! (`docs/combat/stats.md` §"Direct attributes (8)"). These are AUTHORED on a
//! [`GangerSpawn`](crate::situation::GangerSpawn); the COMPUTED combat stats
//! ([`Shooting`](crate::ganger::Shooting) / [`Tu`](crate::ganger::Tu) /
//! [`Hp`](crate::ganger::Hp) / … and the dormant Fight / Reactions / Morale / Bottle)
//! are DERIVED from them at setup via
//! [`derive_stats`](crate::ganger::derive_stats) × the
//! [`GangerStatTuning`](crate::tuning::GangerStatTuning) weights (GTW-384, the
//! two-layer model).
//!
//! Six of the eight live HERE — [`Speed`], [`Aim`], [`Strength`], [`Reflexes`],
//! [`Cool`], [`Grit`]. The remaining two — [`Toughness`](crate::ganger::Toughness)
//! and [`Luck`](crate::ganger::Luck) — already existed as components (the §6 severity
//! roll reads them), so they stay in [`vitals`](crate::ganger::vitals) and are REUSED,
//! never duplicated.
//!
//! Each is a named newtype over a private inner `f32` with a derived
//! [`Deref`](bevy::prelude::Deref) (house style) and `#[serde(transparent)]` so an
//! authored situation `.ron` names it as a bare scalar. Dimensionless — **zero
//! pixels** (the sim is presentation-agnostic). Defaults to `0.0` (the structural
//! spawn default; the per-ganger value comes from the situation).

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

use crate::ganger::vitals::{Luck, Toughness};

/// A ganger's **Speed** direct attribute — quickness, literally how fast they are
/// physically (`docs/combat/stats.md`).
///
/// The dominant driver of the derived [`Tu`](crate::ganger::Tu) action budget
/// (`tu_base + tu_per_speed·Speed`) and a weighted term in the derived
/// [`Fight`](crate::ganger::Fight) and [`Reactions`](crate::ganger::Reactions) stats.
/// A raw authored potential (slowly-changing). A distinct component so the
/// derivation can query `&Speed` alone. Defaults to `0.0`. `#[serde(transparent)]`
/// lets an authored Speed parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Speed(f32);

impl Speed {
    /// Build a Speed attribute from its magnitude (dimensionless; higher = faster).
    #[must_use]
    pub const fn new(speed: f32) -> Self {
        Self(speed)
    }
}

/// A ganger's **Aim** direct attribute — innate marksmanship, a measure of
/// hand-eye coordination (`docs/combat/stats.md`).
///
/// The dominant weighted term in the derived [`Shooting`](crate::ganger::Shooting)
/// stat (`aim_w·Aim + reflexes_w·Reflexes + cool_w·Cool`), which feeds the §1b shot
/// concentration. A raw authored potential. A distinct component so the derivation
/// can query `&Aim` alone. Defaults to `0.0`. `#[serde(transparent)]` lets an
/// authored Aim parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Aim(f32);

impl Aim {
    /// Build an Aim attribute from its magnitude (dimensionless; higher = better aim).
    #[must_use]
    pub const fn new(aim: f32) -> Self {
        Self(aim)
    }
}

/// A ganger's **Strength** direct attribute — physical power; melee force;
/// carrying capacity (`docs/combat/stats.md`).
///
/// A weighted term in the derived [`Fight`](crate::ganger::Fight) melee stat. (The
/// stats.md over-encumbrance TU reduction is NOT yet built; this slice uses Strength
/// only in the Fight derivation.) A raw authored potential. A distinct component so
/// the derivation can query `&Strength` alone. Defaults to `0.0`.
/// `#[serde(transparent)]` lets an authored Strength parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Strength(f32);

impl Strength {
    /// Build a Strength attribute from its magnitude (dimensionless; higher =
    /// stronger).
    #[must_use]
    pub const fn new(strength: f32) -> Self {
        Self(strength)
    }
}

/// A ganger's **Reflexes** direct attribute — reaction speed, twitchiness
/// (`docs/combat/stats.md`).
///
/// A weighted term in the derived [`Shooting`](crate::ganger::Shooting) and
/// [`Reactions`](crate::ganger::Reactions) stats. A raw authored potential. A
/// distinct component so the derivation can query `&Reflexes` alone. Defaults to
/// `0.0`. `#[serde(transparent)]` lets an authored Reflexes parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Reflexes(f32);

impl Reflexes {
    /// Build a Reflexes attribute from its magnitude (dimensionless; higher =
    /// twitchier).
    #[must_use]
    pub const fn new(reflexes: f32) -> Self {
        Self(reflexes)
    }
}

/// A ganger's **Cool** direct attribute — nerves under fire, "how good are you at
/// keeping your cool" (`docs/combat/stats.md`).
///
/// The broadest contributor: it feeds (at varying weights) the derived
/// [`Shooting`](crate::ganger::Shooting), [`Fight`](crate::ganger::Fight),
/// [`Reactions`](crate::ganger::Reactions), [`Hp`](crate::ganger::Hp) (a little —
/// the stats.md ~0.5 weight), and [`Morale`](crate::ganger::Morale) derivations. A
/// raw authored potential. A distinct component so the derivation can query `&Cool`
/// alone. Defaults to `0.0`. `#[serde(transparent)]` lets an authored Cool parse as
/// a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cool(f32);

impl Cool {
    /// Build a Cool attribute from its magnitude (dimensionless; higher = steadier
    /// nerves).
    #[must_use]
    pub const fn new(cool: f32) -> Self {
        Self(cool)
    }
}

/// A ganger's **Grit** direct attribute — resilience, the ability to keep going in
/// the face of pain or terrible odds (`docs/combat/stats.md`).
///
/// The dominant weighted term in the derived [`Hp`](crate::ganger::Hp) and
/// [`Morale`](crate::ganger::Morale) stats (the damage / psychological pools), and a
/// term in [`Fight`](crate::ganger::Fight). A raw authored potential. A distinct
/// component so the derivation can query `&Grit` alone. Defaults to `0.0`.
/// `#[serde(transparent)]` lets an authored Grit parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Grit(f32);

impl Grit {
    /// Build a Grit attribute from its magnitude (dimensionless; higher = more
    /// resilient).
    #[must_use]
    pub const fn new(grit: f32) -> Self {
        Self(grit)
    }
}

/// The eight **direct attributes** of one ganger, grouped by value for the
/// [`derive_stats`](crate::ganger::derive_stats) pure derivation (GTW-384).
///
/// A render-free input record (no-bare-types: each field is its named attribute
/// newtype, never a bare `f32`). [`setup_battle`](crate::situation::setup_battle)
/// builds one from an authored [`GangerSpawn`](crate::situation::GangerSpawn)'s eight
/// authored attributes and the hot-reload re-derive reconstructs one from the spawned
/// ganger's attribute components — both feed it to `derive_stats`, the single source
/// of truth for the computed combat stats.
///
/// Six attributes ([`Speed`] / [`Aim`] / [`Strength`] / [`Reflexes`] / [`Cool`] /
/// [`Grit`]) live in this module; [`Toughness`] and [`Luck`] are the two REUSED
/// existing components (the §6 severity roll reads them), carried here so the HP
/// derivation can weight Toughness without the caller juggling two sources.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GangerAttributes {
    /// Quickness — drives the derived TU budget + Fight/Reactions terms.
    pub speed:     Speed,
    /// Marksmanship — the dominant Shooting term.
    pub aim:       Aim,
    /// Physical power — a Fight term.
    pub strength:  Strength,
    /// Damage resistance — a (reused) severity-roll term, and an HP-derivation term.
    pub toughness: Toughness,
    /// Reaction speed — a Shooting + Reactions term.
    pub reflexes:  Reflexes,
    /// Nerves under fire — the broad Shooting/Fight/Reactions/HP/Morale contributor.
    pub cool:      Cool,
    /// Resilience — the dominant HP + Morale term, and a Fight term.
    pub grit:      Grit,
    /// Directional fortune — the (reused) severity-roll tail modulator; feeds the
    /// severity roll ONLY, never the computed stats (`docs/combat/stats.md`).
    pub luck:      Luck,
}

// `GangerAttributes` is constructed via its struct literal (`GangerAttributes { speed,
// aim, … }`) — its eight fields are public NAMED-NEWTYPE values, so a literal is
// self-documenting and there is no bare-type leak. A flat eight-arg `new` would trip
// clippy `too_many_arguments` (fires at 8), and grouping the eight attributes for a `new`
// would just re-describe THIS group — so the literal IS the constructor.
