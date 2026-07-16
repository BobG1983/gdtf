//! The ganger stat scalar newtypes — the two-pool vitals + identity a
//! [`GangerView`](super::GangerView) carries (GTW-734).
//!
//! The wire mirrors of the sim's `Hp`/`HpMax`, `Wounds`/`WoundsMax`, `Tu`/`TuMax`,
//! `Faction`, `GangerName`. Each keeps the sim's inner width (`u16` HP, `u8` for the
//! small pools) and is a distinct newtype per no-bare-types rule 3 (a current pool is
//! not its ceiling; HP is not Wounds). Private inner + derived `Deref`, serde-
//! transparent.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// A ganger's current **hit points** — the knock-down pool (`HP ≤ 0` → downed). Mirror
/// of the sim `Hp` (`u16`).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HpNet(u16);

impl HpNet {
    /// Build a current-HP value from its count.
    #[must_use]
    pub const fn new(hp: u16) -> Self {
        Self(hp)
    }
}

/// A ganger's **HP ceiling** — the HP bar's denominator. Mirror of the sim `HpMax`
/// (`u16`); a distinct newtype from [`HpNet`] (rule 3).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HpMaxNet(u16);

impl HpMaxNet {
    /// Build an HP-ceiling value from its count.
    #[must_use]
    pub const fn new(hp_max: u16) -> Self {
        Self(hp_max)
    }
}

/// A ganger's current **Wounds** — the small life pool (`Wounds ≤ 0` → dead). Mirror of
/// the sim `Wounds` (`u8`).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WoundsNet(u8);

impl WoundsNet {
    /// Build a current-Wounds value from its count.
    #[must_use]
    pub const fn new(wounds: u8) -> Self {
        Self(wounds)
    }
}

/// A ganger's **Wounds ceiling** — the pip count. Mirror of the sim `WoundsMax` (`u8`);
/// a distinct newtype from [`WoundsNet`] (rule 3).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WoundsMaxNet(u8);

impl WoundsMaxNet {
    /// Build a Wounds-ceiling value from its count.
    #[must_use]
    pub const fn new(wounds_max: u8) -> Self {
        Self(wounds_max)
    }
}

/// A ganger's current **Time Units** — the per-turn action budget. Mirror of the sim
/// `Tu` (`u8`).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TuNet(u8);

impl TuNet {
    /// Build a current-TU value from its count.
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

/// A ganger's **TU ceiling** — the round-start budget the pool resets to. Mirror of the
/// sim `TuMax` (`u8`); a distinct newtype from [`TuNet`] (rule 3).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TuMaxNet(u8);

impl TuMaxNet {
    /// Build a TU-ceiling value from its count.
    #[must_use]
    pub const fn new(tu_max: u8) -> Self {
        Self(tu_max)
    }
}

/// A gang (faction) identity — which side a ganger fights for. Mirror of the sim
/// `Faction` (`u8` gang index).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FactionNet(u8);

impl FactionNet {
    /// Build a faction identity from its gang index.
    #[must_use]
    pub const fn new(gang: u8) -> Self {
        Self(gang)
    }
}

/// A ganger's display **name**. Mirror of the sim `GangerName` (`String`); a name
/// newtype, serde-transparent. `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GangerNameNet(String);

impl GangerNameNet {
    /// Build a ganger name from its display string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}
