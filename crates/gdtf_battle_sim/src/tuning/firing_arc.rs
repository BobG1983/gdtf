//! The GTW-242 firing arc — the angular width of the facing cone a shooter may
//! fire within before it must turn to face the target.

use bevy::prelude::Deref;
use serde::Deserialize;

/// The **firing arc** — the full angular width (in **degrees**) of the facing cone a
/// shooter may fire within before it must turn to face the target (GTW-242).
///
/// A shot whose target lies within `±firing_arc / 2` of the shooter's facing fires
/// directly; a target outside that arc requires the shooter to first turn to face it
/// (paying the turn TU) AND afford the shot — else the shot is rejected
/// (`docs/combat/resolution.md` §1 / the targeting section). A GLOBAL arc (one width for
/// every weapon this slice; a per-weapon arc — pistol wide / rifle narrow — is a flagged
/// future refinement). The arc is a **continuous angle**, tested against the true angle
/// between the facing unit vector and the actor→target ground vector — NOT 8-way snapping.
///
/// A distinct domain concept ⇒ its own newtype (no-bare-types rule 3): a wrapped angle in
/// degrees, never a bare `f32`. Private inner + derived [`Deref`]; `#[serde(transparent)]`
/// lets it parse a bare RON scalar (the tuning-leaf precedent). The default is `120.0`
/// (USER DECISION 2026-06-16: "probably something like 120 degree arc") — **tunable**
/// balance data; tests assert only the relation to this value, never the magnitude.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FiringArc(f32);

impl FiringArc {
    /// Build a firing arc from its full angular width in **degrees** (tunable balance
    /// data).
    ///
    /// The constructor for the newtype — keeps the inner `f32` private (house style)
    /// while letting tests and any programmatic tuning edit build an arc without a bare
    /// `f32` escaping; shipped values come from the `.ron` via the derived
    /// [`Deserialize`].
    #[must_use]
    pub const fn new(degrees: f32) -> Self {
        Self(degrees)
    }
}

impl Default for FiringArc {
    fn default() -> Self {
        // 120° full width (±60° off the facing) — USER DECISION 2026-06-16: "probably
        // something like 120 degree arc". Tunable balance data; value-agnostic tests
        // only, never a pinned magnitude.
        Self(120.0)
    }
}
