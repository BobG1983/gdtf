//! The authored vertical-link shape — the [`OneWay`] directionality flag, the
//! [`LinkKind`] (stair / ladder) enum, and the two-endpoint [`VerticalLink`] record
//! an authored situation deserialises.

use bevy::prelude::Deref;
use serde::Deserialize;

use crate::metric::CellLevel;

/// Whether a [`VerticalLink`]'s kind is **one-way** — traversable only in the
/// authored `(from → to)` direction.
///
/// A named newtype over `bool` (no-bare-types: a link's directionality is a
/// load-bearing domain value — it gates whether
/// [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph) indexes the reverse
/// direction — not a bare boolean). `true` means one-way (forward only); `false`
/// (the default) means bidirectional (climbed both up and down). Private inner +
/// derived [`Deref`] (house style, matching `Aiming`/`Destroyed`).
/// `#[serde(transparent)]` lets an authored directionality parse as a bare boolean.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct OneWay(bool);

impl OneWay {
    /// Build a directionality flag from its boolean state — `true` for a one-way
    /// link, `false` for a bidirectional one.
    #[must_use]
    pub const fn new(one_way: bool) -> Self {
        Self(one_way)
    }

    /// A bidirectional link — traversable in both directions (the default).
    #[must_use]
    pub const fn bidirectional() -> Self {
        Self(false)
    }

    /// A one-way link — traversable only in the authored `(from → to)` direction.
    #[must_use]
    pub const fn forward_only() -> Self {
        Self(true)
    }

    /// Whether this flag denotes a one-way link.
    #[must_use]
    pub const fn is_one_way(self) -> bool {
        self.0
    }
}

/// What kind of authored vertical connection a [`VerticalLink`] is, and whether
/// it may be traversed in both directions.
///
/// A named domain enum (no-bare-types: a link's kind is a domain value, not a
/// bare bool/`u8`). The two physical kinds are the ones `docs/combat/combat.md`
/// names — **stair** and **ladder** — each carrying a [`OneWay`] flag. By default
/// a link is **bidirectional** (a stair/ladder is climbed both up and down); a
/// one-way link (e.g. a drop a ganger can descend but not climb back) is
/// traversable only from its lower-listed endpoint to its higher-listed one (the
/// authored `(from → to)` direction). Derives [`Deserialize`] so an authored
/// situation names its links' kinds (`Stair`/`Ladder`) and one-way flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum LinkKind {
    /// A staircase between storeys.
    Stair {
        /// Whether the stair is traversable only in the authored `(from → to)`
        /// direction ([`OneWay::forward_only`]) or bidirectional ([`OneWay::bidirectional`], the default).
        one_way: OneWay,
    },
    /// A ladder between storeys.
    Ladder {
        /// Whether the ladder is traversable only in the authored `(from → to)`
        /// direction ([`OneWay::forward_only`]) or bidirectional ([`OneWay::bidirectional`], the default).
        one_way: OneWay,
    },
}

impl LinkKind {
    /// A normal, bidirectional staircase.
    #[must_use]
    pub const fn stair() -> Self {
        Self::Stair {
            one_way: OneWay::bidirectional(),
        }
    }

    /// A normal, bidirectional ladder.
    #[must_use]
    pub const fn ladder() -> Self {
        Self::Ladder {
            one_way: OneWay::bidirectional(),
        }
    }

    /// Whether this kind is one-way — traversable only in the authored
    /// `(from → to)` direction. `false` means bidirectional (climbed both ways).
    #[must_use]
    pub const fn is_one_way(self) -> bool {
        match self {
            Self::Stair { one_way } | Self::Ladder { one_way } => one_way.is_one_way(),
        }
    }
}

/// One authored vertical connection between two storeys — its two `(cell, level)`
/// endpoints and its [`LinkKind`].
///
/// A named struct rather than a bare `(CellLevel, CellLevel, LinkKind)` tuple so
/// the authored shape is self-describing. The endpoints are the GTW-151
/// [`CellLevel`] pair: [`from`](VerticalLink::from) is the authored departure
/// `(cell, level)`, [`to`](VerticalLink::to) the authored arrival. For a
/// bidirectional kind the link is traversable both ways; for a
/// [`one-way`](LinkKind::is_one_way) kind only `from → to`. Derives
/// [`Deserialize`] so an authored situation names each link's two endpoints + kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct VerticalLink {
    /// The authored departure endpoint — `(cell, level_from)`.
    pub from: CellLevel,
    /// The authored arrival endpoint — `(cell, level_to)`.
    pub to:   CellLevel,
    /// The kind of connection (stair / ladder, with its [`OneWay`] flag).
    pub kind: LinkKind,
}

impl VerticalLink {
    /// Build a vertical link from its two `(cell, level)` endpoints and its
    /// [`LinkKind`].
    #[must_use]
    pub const fn new(from: CellLevel, to: CellLevel, kind: LinkKind) -> Self {
        Self { from, to, kind }
    }
}
