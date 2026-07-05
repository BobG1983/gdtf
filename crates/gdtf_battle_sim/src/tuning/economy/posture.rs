//! Posture/facing-change TU costs (the E4.1 `set_stance` / `set_facing` verbs):
//! [`StanceChangeTu`] and [`TurnTu`].

use bevy::prelude::Deref;
use serde::Deserialize;

/// The **stance-change TU cost** — the flat number of Time Units a ganger spends to
/// change posture (`docs/combat/combat.md` L34 lists "kneel" among the actions that
/// "cost TUs"; `docs/combat/resolution.md` §"What's tunable" names "stance-change TU").
///
/// The flat cost charged by the E4.1 [`crate::posture::set_stance`] verb — spent via
/// [`crate::tu::spend_tu`] **only when the stance actually changes** (re-asserting the
/// posture a ganger already holds is a no-op, no charge). A small `u8` count, matching
/// [`crate::ganger::Tu`]'s inner type so the economy subtracts it directly. The default
/// is a **starting point**, tunable balance data — tests assert only the relation to
/// this value (the drop equals it), never the magnitude. `#[serde(transparent)]` lets
/// it parse a bare RON scalar; private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct StanceChangeTu(u8);

impl StanceChangeTu {
    /// Build a stance-change TU cost from its flat Time-Unit magnitude (a starting
    /// point, TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house style)
    /// while letting the `posture` tests and any programmatic tuning edit build a cost
    /// without a bare `u8` escaping; shipped values come from the `.ron` via the derived
    /// [`Deserialize`].
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for StanceChangeTu {
    fn default() -> Self {
        // A flat 8 TU to change stance — a STARTING POINT (tunable balance data).
        // `set_stance` spends it only when the posture actually changes; value-agnostic
        // tests only, never a pinned magnitude.
        Self(8)
    }
}

/// The **turn TU cost** — the number of Time Units a ganger spends **per 45deg step**
/// when turning in place toward a new facing (`docs/combat/combat.md` L34 affirmatively
/// lists "turn" among the actions that "cost TUs").
///
/// The per-step cost charged by the E4.1 [`crate::posture::set_facing`] verb — spent via
/// [`crate::tu::spend_tu`] once for each whole 45deg step it can afford (PARTIAL turn: it
/// lands partway when the pool runs out, and re-asserting the facing a ganger already
/// holds is a no-op, no charge). The docs do not fix the *magnitude* (resolution.md
/// §"What's tunable" lists turn TU as tunable); the chosen value is `1` per step (USER
/// DECISION 2026-06-16: "turn costs 1 TU per facing change"). A small `u8` count, matching
/// [`crate::ganger::Tu`]'s inner type so the economy subtracts it directly. The default is
/// tunable balance data — tests assert only the relation to this value, never the
/// magnitude. `#[serde(transparent)]` lets it parse a bare RON scalar; private inner +
/// derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct TurnTu(u8);

impl TurnTu {
    /// Build a turn TU cost from its per-45deg-step Time-Unit magnitude (tunable balance
    /// data).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house style)
    /// while letting the `posture` tests and any programmatic tuning edit build a cost
    /// without a bare `u8` escaping; shipped values come from the `.ron` via the derived
    /// [`Deserialize`].
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for TurnTu {
    fn default() -> Self {
        // 1 TU per 45deg step (USER DECISION 2026-06-16: "turn costs 1 TU per facing
        // change") — tunable balance data. `set_facing` spends it once per afforded step
        // (partial turn); value-agnostic tests only, never a pinned magnitude.
        Self(1)
    }
}
