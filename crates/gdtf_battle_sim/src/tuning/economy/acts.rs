//! Deliberate context-act flat TU costs: [`ShoveTu`], [`OpenDoorTu`],
//! [`EnterEmplacementTu`], [`ExitEmplacementTu`], and [`ThrowTu`].

use bevy::prelude::Deref;
use serde::Deserialize;

/// The **shove TU cost** — the flat number of Time Units a ganger spends to perform the
/// deliberate SHOVE act (GTW-525): a pure-displacement melee shove that knocks an adjacent
/// opposing ganger back one cell (the fall, if any, does the damage — the shove itself
/// deals no wound).
///
/// The flat cost charged by the [`dispatch_shove`](crate::acts::dispatch_shove) act via
/// [`crate::tu::spend_tu`] whenever the deliberate shove RESOLVES its gates (8-adjacency +
/// opposing + alive). A shove costs TU whether or not the displacement lands a fall — the
/// swing of a shove is spent regardless (the melee `fight-mode TU` / ranged `fire()`
/// charge precedent). The WEAPON-TAG auto-shove (on a connecting attack) is FREE — it
/// rides the attack's own TU charge (the tag adds a bundled effect, not a second act), so
/// this leaf is read ONLY by the deliberate act. A small `u8` count, matching
/// [`crate::ganger::Tu`]'s inner type so the economy subtracts it directly. The default is
/// a **starting point**, tunable balance data — tests assert only the relation to this
/// value (the drop equals it), never the magnitude. `#[serde(transparent)]` lets it parse a
/// bare RON scalar; private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ShoveTu(u8);

impl ShoveTu {
    /// Build a shove TU cost from its flat Time-Unit magnitude (a starting point, TBD
    /// tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house style) while
    /// letting the shove-act tests and any programmatic tuning edit build a cost without a
    /// bare `u8` escaping; shipped values come from the `.ron` via the derived
    /// [`Deserialize`].
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for ShoveTu {
    fn default() -> Self {
        // A flat 6 TU to shove — a STARTING POINT (tunable balance data): a real cost (a
        // shove is a committed melee action) but cheaper than a full swing since it deals no
        // wound of its own. Value-agnostic tests only, never a pinned magnitude.
        Self(6)
    }
}

/// The **open-door TU cost** — the flat number of Time Units a ganger spends to perform the
/// deliberate OPEN-DOOR act (GTW-315): a manual interaction that flips an adjacent CLOSED
/// openable piece (a door / hatch) to open, clearing its path + vision block.
///
/// The flat cost charged by the [`dispatch_open_door`](crate::acts::dispatch_open_door) act via
/// [`crate::tu::spend_tu`] whenever the open-door gates RESOLVE (an 8-adjacent CLOSED door +
/// an actor that can afford it). Opening a door is a quick, uncontested action — no roll, no
/// wound — so it is cheaper than a committed melee [`ShoveTu`] swing; the chosen default is a
/// flat `4` TU, matched to the cheapest move / single-link "one simple action" baseline
/// ([`MoveCosts`](super::MoveCosts)'s `open` / [`LinkTu`](super::LinkTu)). A small `u8` count, matching [`crate::ganger::Tu`]'s
/// inner type so the economy subtracts it directly. The default is a **starting point**,
/// tunable balance data — tests assert only the relation to this value (the drop equals it),
/// never the magnitude. `#[serde(transparent)]` lets it parse a bare RON scalar; private inner
/// + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct OpenDoorTu(u8);

impl OpenDoorTu {
    /// Build an open-door TU cost from its flat Time-Unit magnitude (a starting point, TBD
    /// tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house style) while
    /// letting the open-door-act tests and any programmatic tuning edit build a cost without a
    /// bare `u8` escaping; shipped values come from the `.ron` via the derived [`Deserialize`].
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for OpenDoorTu {
    fn default() -> Self {
        // A flat 4 TU to open an adjacent door — a STARTING POINT (tunable balance data): a real
        // cost (a deliberate act) but cheaper than a shove (6) since opening a door is a quick,
        // uncontested interaction that rolls nothing and wounds no one. Matched to the cheapest
        // move-cost baseline / single link hop (one simple action's worth of effort).
        // Value-agnostic tests only, never a pinned magnitude.
        Self(4)
    }
}

/// The **enter-emplacement TU cost** — the flat number of Time Units a ganger spends to ENTER
/// (man) an adjacent weapon emplacement (GTW-543): a deliberate context action that seats the
/// ganger at the mounted gun (setting the emplacement
/// [`Occupied`](crate::terrain::emplacement::EmplacementState::Occupied) and forcing the
/// occupant to read as HIGH cover).
///
/// The flat cost the enter act charges via [`crate::tu::spend_tu`] when its gates resolve (an
/// 8-adjacent VACANT emplacement + an actor that can afford it). Entering a mounted position —
/// swinging into the seat, gripping the gun — is a deliberate committed setup, so it is
/// pricier than the quick uncontested [`OpenDoorTu`] door interaction; the chosen default is a
/// flat `6` TU (matched to the committed-melee [`ShoveTu`] baseline). A small `u8` count,
/// matching [`crate::ganger::Tu`]'s inner type so the economy subtracts it directly. The
/// default is a **starting point**, tunable balance data — tests assert only the relation to
/// this value (the drop equals it), never the magnitude. `#[serde(transparent)]` lets it parse
/// a bare RON scalar; private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct EnterEmplacementTu(u8);

impl EnterEmplacementTu {
    /// Build an enter-emplacement TU cost from its flat Time-Unit magnitude (a starting point,
    /// TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house style) while
    /// letting the enter-act tests and any programmatic tuning edit build a cost without a bare
    /// `u8` escaping; shipped values come from the `.ron` via the derived [`Deserialize`].
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for EnterEmplacementTu {
    fn default() -> Self {
        // A flat 6 TU to man an adjacent emplacement — a STARTING POINT (tunable balance data):
        // a deliberate committed setup (swing into the seat, grip the gun), pricier than a quick
        // door open (4) and matched to the committed-melee shove (6). Value-agnostic tests only,
        // never a pinned magnitude.
        Self(6)
    }
}

/// The **exit-emplacement TU cost** — the flat number of Time Units a ganger spends to EXIT
/// (dismount) the weapon emplacement it is manning (GTW-543): a SEPARATE deliberate context
/// action (there is NO force-eject — a ganger leaves the mount only by spending this cost),
/// setting the emplacement [`Vacant`](crate::terrain::emplacement::EmplacementState::Vacant)
/// and restoring the occupant's stance-derived cover band.
///
/// The flat cost the exit act charges via [`crate::tu::spend_tu`] when it resolves (the actor
/// is the current occupant that can afford it). Dismounting — unclamping, stepping clear — is a
/// quicker action than the committed enter, so the chosen default is a flat `4` TU (matched to
/// the cheapest one-simple-action baseline, like [`OpenDoorTu`]). A small `u8` count, matching
/// [`crate::ganger::Tu`]'s inner type. The default is a **starting point**, tunable balance
/// data — tests assert only the relation to this value, never the magnitude.
/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ExitEmplacementTu(u8);

impl ExitEmplacementTu {
    /// Build an exit-emplacement TU cost from its flat Time-Unit magnitude (a starting point,
    /// TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house style) while
    /// letting the exit-act tests and any programmatic tuning edit build a cost without a bare
    /// `u8` escaping; shipped values come from the `.ron` via the derived [`Deserialize`].
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for ExitEmplacementTu {
    fn default() -> Self {
        // A flat 4 TU to dismount an emplacement — a STARTING POINT (tunable balance data):
        // quicker than manning it (6), matched to the cheapest one-simple-action baseline (like
        // OpenDoorTu). Value-agnostic tests only, never a pinned magnitude.
        Self(4)
    }
}

/// The **throw-grenade TU cost** — the flat number of Time Units a ganger spends to LOB a
/// grenade / grenade-launcher charge at a target cell (GTW-546, child GTW-41d): the deliberate
/// blind-throw context action, resolving the arc march + the GTW-541 blast at the landing.
///
/// The flat cost the throw act charges via [`crate::tu::spend_tu`] when its gates resolve (the
/// thrower wields a [`TrajectoryStyle::Arc`](crate::weapon::TrajectoryStyle) weapon with a
/// loaded round + can afford it). Priming and lobbing a grenade is a committed one-action
/// throw, so the chosen default is a flat `6` TU (matched to the committed-setup
/// [`EnterEmplacementTu`] / [`ShoveTu`] baseline). A small `u8` count, matching
/// [`crate::ganger::Tu`]'s inner type so the economy subtracts it directly. The default is a
/// **starting point**, tunable balance data — tests assert only the relation to this value
/// (the drop equals it), never the magnitude. `#[serde(transparent)]` lets it parse a bare RON
/// scalar; private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ThrowTu(u8);

impl ThrowTu {
    /// Build a throw-grenade TU cost from its flat Time-Unit magnitude (a starting point, TBD
    /// tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house style) while
    /// letting the throw-act tests and any programmatic tuning edit build a cost without a bare
    /// `u8` escaping; shipped values come from the `.ron` via the derived [`Deserialize`].
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for ThrowTu {
    fn default() -> Self {
        // A flat 6 TU to lob a grenade — a STARTING POINT (tunable balance data): a committed
        // one-action throw (prime + lob), matched to the committed-setup enter-emplacement (6)
        // and shove (6) baselines. Value-agnostic tests only, never a pinned magnitude.
        Self(6)
    }
}
