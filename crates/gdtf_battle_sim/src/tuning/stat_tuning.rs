//! The [`GangerStatTuning`] resource — the attribute → computed-stat derivation
//! weights / divisors / TU params (`docs/combat/stats.md` §"Computed combat stats").
//!
//! A SEPARATE tuning store from [`CombatTuning`](crate::tuning::CombatTuning) (the
//! user-directed split): combat tuning holds the *resolution* coefficients, this holds
//! the *character-sheet* derivation. It is its own Bevy [`Resource`], serde-loaded from
//! `assets/combat/stat_tuning.ron` and hot-reloadable exactly like `CombatTuning`
//! (GTW-374 pattern, GTW-384).
//!
//! Every numeric leaf is a named newtype with a PRIVATE inner (no-bare-types rule 5):
//! the per-derived-stat attribute [`StatWeight`]s, the two life-pool divisors
//! ([`WoundsPerHp`] / [`BottlePerMorale`]), and the TU params ([`TuBase`] /
//! [`TuPerSpeed`]). Each carries a derived [`Deref`](bevy::prelude::Deref) +
//! `#[serde(transparent)]` so it round-trips as a bare RON scalar. The derivation
//! [`derive_stats`](crate::ganger::derive_stats) GENUINELY CONSUMES every leaf (no dead
//! leaf — the GTW-364 C7 lesson); the per-stat grouping structs spell out exactly where.

use bevy::{
    prelude::{Deref, Resource},
    reflect::TypePath,
};
use serde::Deserialize;

/// A single attribute → computed-stat **weight** — the multiplier on one direct
/// attribute in a weighted-sum derivation (`docs/combat/stats.md`: "the derivations are
/// weighted attribute sums; … defaults are flat 1.0 weights, Cool at 0.5 into HP").
///
/// A named newtype over a private `f32` (no-bare-types). Distinct from the resolution
/// scalars in [`CombatTuning`](crate::tuning::CombatTuning); this is a character-sheet
/// derivation weight. Defaults to `1.0` (a flat full contribution — the stats.md
/// default); a group's [`Default`] overrides the few that differ (e.g. Cool 0.5 into HP).
/// `#[serde(transparent)]` lets an authored weight parse as a bare scalar.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct StatWeight(f32);

impl StatWeight {
    /// Build a derivation weight from its magnitude (dimensionless; the multiplier on
    /// one attribute in a weighted-sum derived stat).
    #[must_use]
    pub const fn new(weight: f32) -> Self {
        Self(weight)
    }
}

impl Default for StatWeight {
    /// A flat full contribution (`1.0`) — the stats.md default weight.
    fn default() -> Self {
        Self(1.0)
    }
}

/// The **HP per Wound** divisor — `Wounds = round(HP / wounds_per_hp)`
/// (`docs/combat/stats.md`: "`HP / wounds_per_hp` (tunable, ≈10)"). A tougher/steadier
/// ganger gets a proportionally larger life pool (the second derivation level).
///
/// A named newtype over a private `f32` (no-bare-types). Defaults to `10.0` (the
/// stats.md ≈10). `#[serde(transparent)]` lets it parse as a bare scalar. Consumed by
/// [`derive_stats`](crate::ganger::derive_stats) to divide the derived HP into the
/// [`Wounds`](crate::ganger::Wounds) life pool.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct WoundsPerHp(f32);

impl WoundsPerHp {
    /// Build the HP-per-Wound divisor from its magnitude (≈10).
    #[must_use]
    pub const fn new(divisor: f32) -> Self {
        Self(divisor)
    }
}

impl Default for WoundsPerHp {
    /// The stats.md ≈10 divisor.
    fn default() -> Self {
        Self(10.0)
    }
}

/// The **Morale per Bottle** divisor — `Bottle = round(Morale / bottle_per_morale)`
/// (`docs/combat/stats.md`: "`Morale / bottle_per_morale` (tunable, ≈10)") — the
/// psychological mirror of [`WoundsPerHp`].
///
/// A named newtype over a private `f32` (no-bare-types). Defaults to `10.0` (the
/// stats.md ≈10). `#[serde(transparent)]` lets it parse as a bare scalar. Consumed by
/// [`derive_stats`](crate::ganger::derive_stats) to divide the derived Morale into the
/// [`Bottle`](crate::ganger::Bottle) psychological life pool.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BottlePerMorale(f32);

impl BottlePerMorale {
    /// Build the Morale-per-Bottle divisor from its magnitude (≈10).
    #[must_use]
    pub const fn new(divisor: f32) -> Self {
        Self(divisor)
    }
}

impl Default for BottlePerMorale {
    /// The stats.md ≈10 divisor.
    fn default() -> Self {
        Self(10.0)
    }
}

/// The **base TU** floor — the flat Time Units every ganger gets before the Speed term
/// (`docs/combat/stats.md`: "`tu_base + Speed·tu_per_speed`").
///
/// A named newtype over a private `f32` (no-bare-types; the TU pool is integer, this
/// derivation coefficient is `f32` then rounded). Defaults to `30.0`. `#[serde(transparent)]`
/// lets it parse as a bare scalar. Consumed by [`derive_stats`](crate::ganger::derive_stats)
/// as the additive floor of the derived [`Tu`](crate::ganger::Tu) budget.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct TuBase(f32);

impl TuBase {
    /// Build the TU base floor from its magnitude.
    #[must_use]
    pub const fn new(base: f32) -> Self {
        Self(base)
    }
}

impl Default for TuBase {
    /// A base TU floor of `30.0` — authored so a mid-Speed ganger lands near the
    /// pre-GTW-384 ballpark (~60 TU).
    fn default() -> Self {
        Self(30.0)
    }
}

/// The **TU per Speed** coefficient — the Time Units each point of Speed adds
/// (`docs/combat/stats.md`: "`tu_base + Speed·tu_per_speed`").
///
/// A named newtype over a private `f32` (no-bare-types). Defaults to `10.0`.
/// `#[serde(transparent)]` lets it parse as a bare scalar. Consumed by
/// [`derive_stats`](crate::ganger::derive_stats) as the per-Speed slope of the derived
/// [`Tu`](crate::ganger::Tu) budget.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct TuPerSpeed(f32);

impl TuPerSpeed {
    /// Build the per-Speed TU slope from its magnitude.
    #[must_use]
    pub const fn new(slope: f32) -> Self {
        Self(slope)
    }
}

impl Default for TuPerSpeed {
    /// A per-Speed slope of `10.0` — with [`TuBase`] 30, Speed 3 → ~60 TU (the ballpark).
    fn default() -> Self {
        Self(10.0)
    }
}

/// The **Shooting** derivation weights — `Shooting = aim·Aim + reflexes·Reflexes +
/// cool·Cool` (`docs/combat/stats.md`: `fn(Aim, Reflexes, Cool)`).
///
/// Each field is a [`StatWeight`] consumed by [`derive_stats`](crate::ganger::derive_stats)
/// as the multiplier on that attribute — NO dead leaf. The `i / j / k` of the stats.md
/// Shooting formula.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ShootingWeights {
    /// `i` — Aim's weight (the dominant marksmanship term).
    pub aim:      StatWeight,
    /// `j` — Reflexes' weight (steadiness).
    pub reflexes: StatWeight,
    /// `k` — Cool's weight (nerves).
    pub cool:     StatWeight,
}

impl Default for ShootingWeights {
    /// Flat `1.0` weights (the stats.md default) — Aim + Reflexes + Cool contribute equally.
    fn default() -> Self {
        Self {
            aim:      StatWeight::default(),
            reflexes: StatWeight::default(),
            cool:     StatWeight::default(),
        }
    }
}

/// The **Fight** derivation weights — `Fight = speed·Speed + strength·Strength +
/// grit·Grit + cool·Cool` (`docs/combat/stats.md`: `fn(Speed, Strength, Grit, Cool)`).
///
/// Each field is a [`StatWeight`] consumed by [`derive_stats`](crate::ganger::derive_stats)
/// — NO dead leaf.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct FightWeights {
    /// Speed's weight in the melee skill.
    pub speed:    StatWeight,
    /// Strength's weight (melee force).
    pub strength: StatWeight,
    /// Grit's weight (keeping going).
    pub grit:     StatWeight,
    /// Cool's weight (nerves).
    pub cool:     StatWeight,
}

impl Default for FightWeights {
    /// Flat `1.0` weights (the stats.md default).
    fn default() -> Self {
        Self {
            speed:    StatWeight::default(),
            strength: StatWeight::default(),
            grit:     StatWeight::default(),
            cool:     StatWeight::default(),
        }
    }
}

/// The **Reactions** derivation weights — `Reactions = speed·Speed + reflexes·Reflexes +
/// cool·Cool` (`docs/combat/stats.md`: `fn(Speed, Reflexes, Cool)`).
///
/// Each field is a [`StatWeight`] consumed by [`derive_stats`](crate::ganger::derive_stats)
/// — NO dead leaf.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ReactionsWeights {
    /// Speed's weight (quick to respond).
    pub speed:    StatWeight,
    /// Reflexes' weight (the dominant twitch term).
    pub reflexes: StatWeight,
    /// Cool's weight (composure).
    pub cool:     StatWeight,
}

impl Default for ReactionsWeights {
    /// Flat `1.0` weights (the stats.md default).
    fn default() -> Self {
        Self {
            speed:    StatWeight::default(),
            reflexes: StatWeight::default(),
            cool:     StatWeight::default(),
        }
    }
}

/// The **HP** derivation weights — `HP = grit·Grit + toughness·Toughness + cool·Cool`
/// (`docs/combat/stats.md`: `fn(Grit, Toughness, + a little Cool)`).
///
/// Each field is a [`StatWeight`] consumed by [`derive_stats`](crate::ganger::derive_stats)
/// — NO dead leaf. The Cool weight defaults to `0.5` (the explicit stats.md "Cool at 0.5
/// into HP"), the rest flat `1.0`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct HpWeights {
    /// Grit's weight (the dominant resilience term).
    pub grit:      StatWeight,
    /// Toughness's weight (damage resistance).
    pub toughness: StatWeight,
    /// Cool's "a little" weight — the stats.md `0.5` default.
    pub cool:      StatWeight,
}

impl Default for HpWeights {
    /// Grit + Toughness flat `1.0`, Cool `0.5` (the explicit stats.md "Cool at 0.5 into HP").
    fn default() -> Self {
        Self {
            grit:      StatWeight::default(),
            toughness: StatWeight::default(),
            cool:      StatWeight::new(0.5),
        }
    }
}

/// The **Morale** derivation weights — `Morale = grit·Grit + cool·Cool`
/// (`docs/combat/stats.md`: `fn(Grit, Cool)`).
///
/// Each field is a [`StatWeight`] consumed by [`derive_stats`](crate::ganger::derive_stats)
/// — NO dead leaf.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MoraleWeights {
    /// Grit's weight (the dominant resilience term).
    pub grit: StatWeight,
    /// Cool's weight (composure).
    pub cool: StatWeight,
}

impl Default for MoraleWeights {
    /// Flat `1.0` weights (the stats.md default).
    fn default() -> Self {
        Self {
            grit: StatWeight::default(),
            cool: StatWeight::default(),
        }
    }
}

/// The ganger stat-derivation tuning resource — every weight / divisor / TU param the
/// attribute → computed-stat derivation marches with (`docs/combat/stats.md`).
///
/// A Bevy [`Resource`] deserializable from `assets/combat/stat_tuning.ron` — a SEPARATE
/// file from `combat/tuning.ron` (the user-directed split), loaded + hot-reloaded
/// MIRRORING [`CombatTuning`](crate::tuning::CombatTuning) (GTW-374 pattern, GTW-384).
/// Every leaf is GENUINELY CONSUMED by [`derive_stats`](crate::ganger::derive_stats) — no
/// dead leaf (the GTW-364 C7 lesson).
///
/// Derives [`TypePath`] (render-free reflection metadata, no rendering) because the
/// loader wraps it in `RonAsset<T>`, whose payload bound requires `T: TypePath` — the
/// same bound `CombatTuning` satisfies. Defaults are the stats.md
/// flat-`1.0` weights (Cool `0.5` into HP) + ≈10 divisors + a TU base/slope authored so
/// the derived stats land near the pre-GTW-384 ballpark (see the per-leaf docs).
#[derive(Debug, Clone, PartialEq, Default, Resource, Deserialize, TypePath)]
pub struct GangerStatTuning {
    /// The Shooting weights (`Aim, Reflexes, Cool`).
    pub shooting:          ShootingWeights,
    /// The Fight weights (`Speed, Strength, Grit, Cool`).
    pub fight:             FightWeights,
    /// The Reactions weights (`Speed, Reflexes, Cool`).
    pub reactions:         ReactionsWeights,
    /// The HP weights (`Grit, Toughness, Cool`).
    pub hp:                HpWeights,
    /// The Morale weights (`Grit, Cool`).
    pub morale:            MoraleWeights,
    /// The HP-per-Wound divisor (≈10): `Wounds = round(HP / this)`.
    pub wounds_per_hp:     WoundsPerHp,
    /// The Morale-per-Bottle divisor (≈10): `Bottle = round(Morale / this)`.
    pub bottle_per_morale: BottlePerMorale,
    /// The TU base floor: `TU = tu_base + tu_per_speed·Speed`.
    pub tu_base:           TuBase,
    /// The per-Speed TU slope: `TU = tu_base + tu_per_speed·Speed`.
    pub tu_per_speed:      TuPerSpeed,
}
