//! The **fight-mode model** — the melee mirror of the ranged
//! [`fire_mode`](super::super::fire_mode) (GTW-505, child GTW-37a of the GTW-37 melee
//! epic): the per-mode flat [`TuCost`] plus [`Strikes`] count, the closed
//! [`FightModeKind`] and its [`Display`] label, the per-mode [`FightModeSpec`], and the
//! [`FightMode`] selector that lists a melee weapon's offered fight modes.
//!
//! It deliberately MIRRORS the ranged [`FireMode`](super::super::FireMode) triad
//! ([`FireModeSpec`](super::super::FireModeSpec) /
//! [`ModeKind`](super::super::ModeKind)) — a selector newtype over a `Vec` of per-mode
//! specs, each carrying a closed kind plus its per-mode numbers — but DROPS the
//! dispersion machinery: a melee strike has no cone, so there is **no** `cone_mult`,
//! and the per-mode TU is a FLAT [`TuCost`] (not the ranged
//! [`ModeTuPercent`](super::super::ModeTuPercent) pool fraction). The opposed-Fight
//! resolution that CONSUMES these numbers is GTW-506; this slice ships the data + types
//! + serde only.

use std::fmt::Display;

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// A fight mode's **flat TU cost** — the time units one strike action in this mode
/// costs, charged up front (the melee mirror of the ranged
/// [`ModeTuPercent`](super::super::ModeTuPercent), but a FLAT cost, NOT a pool
/// fraction). A melee swing costs a fixed slice of the actor's TU budget rather than a
/// percentage of it.
///
/// A weapon NUMBER (per-fight-mode), a small non-negative TU count (`u16`, the same
/// inner type the ranged TU-economy newtypes use). Private inner + derived [`Deref`];
/// `#[serde(transparent)]` so it parses a bare RON scalar (the
/// [`crate::tuning`] / GTW-200 house style).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TuCost(u16);

impl TuCost {
    /// Build a per-fight-mode flat TU cost from its time-unit count.
    #[must_use]
    pub const fn new(tu: u16) -> Self {
        Self(tu)
    }
}

/// A fight mode's **strike count** — how many strikes the mode lands per fight action
/// (the melee mirror of the ranged [`ModeShots`](super::super::ModeShots)): a single
/// thrust = 1, a flurry of swings > 1. The strike loop the GTW-506 opposed-Fight
/// resolution will run consumes this; this slice carries it as DATA only.
///
/// A weapon NUMBER (per-fight-mode), a small non-negative count (`u16`). Private inner
/// + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Strikes(u16);

impl Strikes {
    /// Build a per-fight-mode strike count from its strikes-per-action number.
    #[must_use]
    pub const fn new(strikes: u16) -> Self {
        Self(strikes)
    }
}

/// A fight mode's **kind** — the closed set of mechanics a melee mode can be: a
/// sweeping [`Swing`](FightModeKind::Swing) or a driving [`Thrust`](FightModeKind::Thrust)
/// (the melee mirror of the ranged [`ModeKind`](super::super::ModeKind) `Single` /
/// `Burst` / `Full` triad — a CLOSED enum, the kind identifying the mode, the
/// human-facing label DERIVED from it via [`Display`], never a stored name string).
///
/// A `Swing` is the wide, sweeping arc (the baseline melee mode — a chainsword's
/// rip, a club's haymaker); a `Thrust` is the focused stab (a knife's lunge, a
/// bayonet's drive). The GTW-506 opposed-Fight resolution will read this kind; this
/// slice only fixes the vocabulary.
///
/// A named domain enum (no-bare-types: a fight-mode kind is a domain value, not a bare
/// `u8`/label string). It is a FIELD value on a [`FightModeSpec`], NOT a
/// `#[derive(Component)]` — the [`FightMode`] selector that holds the specs is the
/// component. `Deserialize` so a melee weapon's authored RON names its mode kind by
/// variant; `Serialize` for the RON round-trip; `Copy`/`Eq`/`Hash` so a
/// [`FightModeSpec`] is `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FightModeKind {
    /// A sweeping arc — the wide baseline melee mode (a chainsword rip, a club swing).
    Swing,
    /// A focused stab — a driving thrust (a knife lunge, a bayonet drive).
    Thrust,
}

impl Display for FightModeKind {
    /// The canonical human-facing mode LABEL the (later) melee picker shows — `"swing"`
    /// / `"thrust"` — derived from the kind (no stored name string), mirroring the
    /// ranged [`ModeKind`](super::super::ModeKind) [`Display`].
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Swing => "swing",
            Self::Thrust => "thrust",
        };
        f.write_str(label)
    }
}

/// One fight mode's per-mode numbers — its kind, flat TU cost, and strike count (the
/// melee mirror of the ranged [`FireModeSpec`](super::super::FireModeSpec), MINUS the
/// `cone_mult` — a melee strike has no dispersion cone).
///
/// A named struct (not a bare tuple) so each per-mode number keeps its [`FightMode`]
/// meaning; every field is a weapon NUMBER newtype or the closed [`FightModeKind`]. The
/// human-facing label comes from [`FightModeKind`]'s [`Display`] (`kind.to_string()`),
/// not a stored string. Every field is `Copy`, so the spec is `Copy` (the
/// [`FireModeSpec`](super::super::FireModeSpec) precedent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FightModeSpec {
    /// Which mode this is — its closed kind (`Swing` / `Thrust`); the human-facing
    /// label is `kind.to_string()`.
    pub kind:    FightModeKind,
    /// The flat TU cost a strike action in this mode charges up front.
    pub tu_cost: TuCost,
    /// How many strikes this mode lands per fight action.
    pub strikes: Strikes,
}

impl FightModeSpec {
    /// Build one fight mode's spec from its kind and its two per-mode numbers.
    #[must_use]
    pub const fn new(kind: FightModeKind, tu_cost: TuCost, strikes: Strikes) -> Self {
        Self {
            kind,
            tu_cost,
            strikes,
        }
    }
}

/// A melee weapon's **fight-mode selector** — the LIST of fight modes a weapon offers,
/// each a [`FightModeSpec`] paired with its closed [`FightModeKind`] (the melee mirror
/// of the ranged [`FireMode`](super::super::FireMode) selector). A weapon offers any
/// subset of `{Swing, Thrust}` in authored order.
///
/// A named newtype over `Vec<`[`FightModeSpec`]`>` (no-bare-types: the selector is a
/// domain value; the inner `Vec` is the collection-of-domain-values carve-out). The
/// private inner + derived [`Deref`] gives slice access (`.iter()` / `.len()` /
/// `.get()`); `#[serde(transparent)]` so it deserializes from a **bare RON list** of
/// mode entries (`fight_mode: [ (kind: Swing, …), … ]`). `Clone`-not-`Copy` (it holds
/// a `Vec`). A `#[derive(Component)]` — the selector lives as a sibling component on
/// the armed melee-weapon entity (its per-mode [`FightModeSpec`] sub-values ride inside
/// it, not as separate components), exactly as the ranged
/// [`FireMode`](super::super::FireMode) does.
///
/// **Invariant:** a well-authored melee weapon lists at least one mode. The code is
/// DEFENSIVE if that is violated — every read has a total fallback and never panics
/// (see [`FightMode::primary`]). `Default` (`FightMode(Vec::new())`, the empty
/// selector) is a **spawn-seed sentinel only** — the `bsn!` spawn path seeds the slot
/// via `Default` before `FightMode::new(..)` overwrites it (the GTW-322 sentinel-Default
/// convention).
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FightMode(Vec<FightModeSpec>);

impl FightMode {
    /// Build a fight-mode selector from its authored list of modes.
    #[must_use]
    pub const fn new(modes: Vec<FightModeSpec>) -> Self {
        Self(modes)
    }

    /// The **primary fight spec** — the FIRST authored mode; else a structural
    /// single-swing default (`Swing`, 0 TU, 1 strike). Returns BY VALUE
    /// ([`FightModeSpec`] is `Copy`). The fallback is TOTAL — NO `unwrap` / `panic`
    /// even for an empty (mis-authored) selector (the ranged
    /// [`FireMode::single`](super::super::FireMode::single) precedent — first authored,
    /// then a structural default).
    #[must_use]
    pub fn primary(&self) -> FightModeSpec {
        if let Some(first) = self.0.first() {
            *first
        } else {
            FightModeSpec::new(FightModeKind::Swing, TuCost::new(0), Strikes::new(1))
        }
    }
}
