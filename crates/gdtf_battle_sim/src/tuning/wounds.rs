//! The wound/bleed/from-Downed cost leaves (E3.6 / E3.7 / E3.8): the per-tier
//! Wounds-budget costs, the bleed-out rate, and the stabilize / execute TU costs.

use bevy::prelude::Deref;
use serde::Deserialize;

/// The Wounds-budget cost of one severity tier — how many [`crate::ganger::Wounds`]
/// a wound at that tier spends from the defender's life pool
/// (`docs/combat/wounds-and-roster.md` §"Severity tiers": Minor 1 / Major 2 /
/// Critical 3).
///
/// One newtype shared by the three per-tier fields of [`WoundCosts`]: each tier's
/// cost is the same *kind* of value (a Wounds-budget spend), distinguished by its
/// field. A small `u8` count, matching [`crate::ganger::Wounds`]'s inner type so
/// the cost subtracts directly from the life pool. `None` costs `0` and `Fatal`
/// **empties** the pool — those two are **structural**, not tuning, so only the
/// three middle tiers are authored here. The `1/2/3` split is a **starting point**
/// (wounds-and-roster.md: "could be 1/2/4, 1/3/5, … TBD (tuning)") — tunable
/// balance data, never pinned by a value test (tests assert only the ordering
/// Minor < Major < Critical). `#[serde(transparent)]` lets it parse a bare RON
/// scalar; private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct WoundCost(u8);

impl WoundCost {
    /// Build a per-tier Wounds-budget cost from its count (TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house
    /// style) while letting callers (e.g. the `wound_cost` helper's tests, or any
    /// code assembling a [`WoundCosts`] outside this module) build a cost without a
    /// bare `u8` escaping.
    #[must_use]
    pub const fn new(cost: u8) -> Self {
        Self(cost)
    }
}

/// The **bleed-out rate** — the flat number of [`crate::ganger::Wounds`] a single
/// un-stabilized [`crate::ganger::LifeState::Downed`] ganger loses **each round**
/// it stays down (`docs/combat/resolution.md` §9; `docs/combat/wounds-and-roster.md`
/// §"Downed → death … state machine").
///
/// The bleed-out clock's per-round drain: once per full round
/// [`crate::effects::bleed::tick_bleed`] subtracts this from every un-stabilized Downed
/// ganger's [`crate::ganger::Wounds`] life pool, and the stack count (turns down)
/// = total Wounds lost — a clock you can read. A small `u8` count, matching
/// [`crate::ganger::Wounds`]'s inner type so it subtracts directly from the life
/// pool (`saturating_sub`, never underflowing). The default `1` is a **starting
/// point**, tunable balance data — tests assert only the relation to this value
/// (the per-tick drop equals it), never the magnitude. The TU/clock economy the
/// drain sits inside (when the tick fires, the execute/stabilize TU costs) is E4.
/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner +
/// derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct BleedRate(u8);

impl BleedRate {
    /// Build a bleed-out rate from its per-round Wounds drain (a starting point,
    /// TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house
    /// style) while letting the bleed-out tests and any programmatic tuning edit
    /// build a rate without a bare `u8` escaping; shipped values come from the
    /// `.ron` via the derived [`Deserialize`].
    #[must_use]
    pub const fn new(rate: u8) -> Self {
        Self(rate)
    }
}

impl Default for BleedRate {
    fn default() -> Self {
        // A flat 1 Wound drained per Downed round — a STARTING POINT (tunable
        // balance data; the TU/clock economy it sits inside is E4), asserted only by
        // its relation to the drain, never as a pinned magnitude.
        Self(1)
    }
}

/// The **stabilize TU cost** — the flat number of Time Units an 8-adjacent ALIVE
/// ally spends to dress a [`crate::ganger::LifeState::Downed`] ganger's wound and
/// halt its bleed-out clock (`docs/combat/resolution.md` §9: `stabilize_downed`
/// "pays the flat `stabilize_tu`"; `docs/combat/wounds-and-roster.md`
/// §"Downed → death … state machine").
///
/// The flat cost of the E3.8 [`crate::acts::downed::stabilize_downed`] verb. **This
/// slice only READS the cost** to wire the leaf — the TU economy (debiting a
/// [`crate::ganger::Tu`] pool, the can-afford check) is **E4**, so nothing here
/// spends or validates against a TU budget. A small `u8` count, matching
/// [`crate::ganger::Tu`]'s inner type so the E4 economy can subtract it directly.
/// The default is a **starting point**, tunable balance data — tests are
/// value-agnostic. `#[serde(transparent)]` lets it parse a bare RON scalar; private
/// inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct StabilizeTu(u8);

impl StabilizeTu {
    /// Build a stabilize TU cost from its flat Time-Unit magnitude (a starting
    /// point, TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house style)
    /// while letting the `downed_acts` tests and any programmatic tuning edit build a
    /// cost without a bare `u8` escaping; shipped values come from the `.ron` via the
    /// derived [`Deserialize`].
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for StabilizeTu {
    fn default() -> Self {
        // A flat 4 TU to stabilize an adjacent ally — a STARTING POINT (tunable
        // balance data). This slice only READS the cost; the TU economy that debits
        // it is E4. Value-agnostic tests only, never a pinned magnitude.
        Self(4)
    }
}

/// The **execute TU cost** — the flat number of Time Units an 8-adjacent ALIVE
/// enemy spends to finish a [`crate::ganger::LifeState::Downed`] ganger outright
/// (`docs/combat/resolution.md` §9: `execute_downed` "pays the flat `execute_tu`";
/// `docs/combat/wounds-and-roster.md` §"Downed → death … state machine").
///
/// The flat cost of the E3.8 [`crate::acts::downed::execute_downed`] verb. **This
/// slice only READS the cost** to wire the leaf — the TU economy (debiting a
/// [`crate::ganger::Tu`] pool, the can-afford check) is **E4**, so nothing here
/// spends or validates against a TU budget. A small `u8` count, matching
/// [`crate::ganger::Tu`]'s inner type so the E4 economy can subtract it directly.
/// The default is a **starting point**, tunable balance data — tests are
/// value-agnostic. `#[serde(transparent)]` lets it parse a bare RON scalar; private
/// inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ExecuteTu(u8);

impl ExecuteTu {
    /// Build an execute TU cost from its flat Time-Unit magnitude (a starting
    /// point, TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house style)
    /// while letting the `downed_acts` tests and any programmatic tuning edit build a
    /// cost without a bare `u8` escaping; shipped values come from the `.ron` via the
    /// derived [`Deserialize`].
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for ExecuteTu {
    fn default() -> Self {
        // A flat 6 TU to execute an adjacent enemy — a STARTING POINT (tunable
        // balance data), a touch dearer than stabilizing. This slice only READS the
        // cost; the TU economy that debits it is E4. Value-agnostic tests only.
        Self(6)
    }
}

/// The per-tier **Wounds-budget costs** — how many [`crate::ganger::Wounds`] each
/// non-structural severity tier spends (`docs/combat/wounds-and-roster.md`
/// §"Severity tiers": Minor 1 / Major 2 / Critical 3).
///
/// Only the three middle tiers are authored: [`crate::severity::Severity::None`]
/// costs `0` and [`crate::severity::Severity::Fatal`] **empties** the pool — both
/// **structural** mechanisms (`apply_hit` branches them directly), not tuning, so
/// they are deliberately absent here. The `1/2/3` split is a **starting point**
/// (wounds-and-roster.md: "could be 1/2/4, 1/3/5, … TBD (tuning)"); the magnitudes
/// are tunable balance data, asserted only by the ordering relation
/// (`minor < major < critical`), never by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct WoundCosts {
    /// The Minor-tier Wounds cost (doc starting point: 1).
    pub minor:    WoundCost,
    /// The Major-tier Wounds cost (doc starting point: 2).
    pub major:    WoundCost,
    /// The Critical-tier Wounds cost (doc starting point: 3).
    pub critical: WoundCost,
}

impl Default for WoundCosts {
    fn default() -> Self {
        // The Minor 1 / Major 2 / Critical 3 starting split from
        // docs/combat/wounds-and-roster.md §"Severity tiers" — a STARTING POINT
        // ("could be 1/2/4, 1/3/5, … TBD (tuning)"), so tunable balance data
        // asserted only by ordering (minor < major < critical), never by value.
        Self {
            minor:    WoundCost(1),
            major:    WoundCost(2),
            critical: WoundCost(3),
        }
    }
}
