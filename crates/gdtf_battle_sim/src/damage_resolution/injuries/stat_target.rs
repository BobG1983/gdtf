//! The [`StatTarget`] discriminant — every stat an injury can modify — and its
//! [`StatKind`] (attribute vs derived) split.

use serde::Deserialize;

/// Every stat an injury can modify — BOTH the eight **direct attributes** AND the
/// eight **derived stats** (`docs/combat/stats.md`, the GTW-384 two-layer model).
///
/// Defined **fresh** in the sim (the presenter's `StatName` is an unrelated UI
/// marker unit-struct, so the sim discriminant is named `StatTarget` to avoid the
/// collision): it is the `stat` selector of an [`InjuryEffect::Modify`](super::InjuryEffect)
/// effect, authored in a `.injury.ron`. The first eight are the authoritative ECS
/// attribute components ([`Speed`](crate::ganger::Speed) … [`Luck`](crate::ganger::Luck)):
/// a delta on one of these folds in PRE-derivation, so it ripples through every
/// derived stat (a `Modify(Aim, -2)` lowers the derived
/// [`Shooting`](crate::ganger::Shooting) automatically). The last eight are the
/// [`derive_stats`](crate::ganger::derive_stats) outputs cached as components: a
/// delta on one of these adds POST-derivation, and for the pools
/// ([`Tu`](crate::ganger::Tu) / [`Hp`](crate::ganger::Hp) /
/// [`Wounds`](crate::ganger::Wounds) / Bottle) it docks the MAX (current clamps via
/// `min`). The projection itself is GTW-436; THIS slice only names the vocabulary.
///
/// A pure value enum (no bare integer / string for the stat axis). Derives
/// [`Hash`] / [`Eq`] so it can key the GTW-436 projector's per-stat lookups and
/// [`Deserialize`] so an authored effect names it as a bare RON identifier.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Deserialize)]
pub enum StatTarget {
    /// Quickness — drives the derived TU budget and Fight / Reactions terms.
    Speed,
    /// Marksmanship — the dominant derived [`Shooting`](crate::ganger::Shooting) term.
    Aim,
    /// Physical power — a derived [`Fight`](crate::ganger::Fight) term.
    Strength,
    /// Damage resistance — the §6 severity-roll term and an HP-derivation term.
    Toughness,
    /// Reaction speed — a derived Shooting / Reactions term.
    Reflexes,
    /// Nerves under fire — the broad Shooting / Fight / Reactions / HP / Morale term.
    Cool,
    /// Resilience — the dominant derived HP / Morale term, and a Fight term.
    Grit,
    /// Directional fortune — the §6 severity-roll tail modulator.
    Luck,
    /// Derived shooting skill — the §1b shot-concentration input.
    Shooting,
    /// Derived melee skill.
    Fight,
    /// Derived reaction skill — the GTW-38 reaction-fire input.
    Reactions,
    /// Derived psychological resolve — the morale pool's basis.
    Morale,
    /// Derived TU action-budget MAX — a `Modify` docks the ceiling, never deals damage.
    Tu,
    /// Derived HP pool MAX — a `Modify` docks the ceiling, never deals damage.
    Hp,
    /// Derived Wounds pool MAX — a `Modify` docks the ceiling, never deals damage.
    Wounds,
    /// Derived Bottle (morale break) pool MAX — a `Modify` docks the ceiling.
    Bottle,
}

/// Whether a [`StatTarget`] delta folds into the attribute (PRE-derivation) or the
/// derived result (POST-derivation).
///
/// The discriminant the GTW-436 projector branches on: an [`Attribute`](StatKind::Attribute)
/// delta is summed onto the base attribute before [`derive_stats`](crate::ganger::derive_stats)
/// runs (so it ripples), a [`Derived`](StatKind::Derived) delta is summed onto the
/// derived output after. A pure value enum (no bare bool for the axis kind).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum StatKind {
    /// One of the eight authoritative attribute components — delta folds PRE-derivation.
    Attribute,
    /// One of the eight derived-stat caches — delta folds POST-derivation.
    Derived,
}

impl StatTarget {
    /// Every [`StatTarget`] in canonical order — the eight direct attributes first,
    /// then the eight derived stats. This is the index order of the per-stat
    /// summed-delta store ([`StatDeltaLedger`](super::StatDeltaLedger)) and the
    /// iteration order the GTW-436 projector reads deltas in.
    pub const ALL: [Self; 16] = [
        Self::Speed,
        Self::Aim,
        Self::Strength,
        Self::Toughness,
        Self::Reflexes,
        Self::Cool,
        Self::Grit,
        Self::Luck,
        Self::Shooting,
        Self::Fight,
        Self::Reactions,
        Self::Morale,
        Self::Tu,
        Self::Hp,
        Self::Wounds,
        Self::Bottle,
    ];

    /// The number of stats — the fixed length of the per-stat summed-delta store.
    pub const COUNT: usize = Self::ALL.len();

    /// Whether this stat is a direct **attribute** (the first eight) or a **derived**
    /// stat (the last eight) — the projector's fold-stage discriminant.
    #[must_use]
    pub const fn kind(self) -> StatKind {
        match self {
            Self::Speed
            | Self::Aim
            | Self::Strength
            | Self::Toughness
            | Self::Reflexes
            | Self::Cool
            | Self::Grit
            | Self::Luck => StatKind::Attribute,
            Self::Shooting
            | Self::Fight
            | Self::Reactions
            | Self::Morale
            | Self::Tu
            | Self::Hp
            | Self::Wounds
            | Self::Bottle => StatKind::Derived,
        }
    }

    /// This stat's index into a sixteen-slot per-stat array (`0..16`), in the same
    /// canonical order as [`ALL`](StatTarget::ALL). Keys the summed-delta store
    /// without a hash map (mirrors [`BodyPart::index`](crate::armor::BodyPart::index)).
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Speed => 0,
            Self::Aim => 1,
            Self::Strength => 2,
            Self::Toughness => 3,
            Self::Reflexes => 4,
            Self::Cool => 5,
            Self::Grit => 6,
            Self::Luck => 7,
            Self::Shooting => 8,
            Self::Fight => 9,
            Self::Reactions => 10,
            Self::Morale => 11,
            Self::Tu => 12,
            Self::Hp => 13,
            Self::Wounds => 14,
            Self::Bottle => 15,
        }
    }
}
