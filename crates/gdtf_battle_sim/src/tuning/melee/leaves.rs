//! The §7 melee opposed-Fight tuning leaves — the roll variance and the
//! margin → damage-multiplier curve coefficients (GTW-506).
//!
//! `docs/combat/resolution.md` §7 (lines 146-150) specifies the melee math
//! exactly:
//!
//! ```text
//! atk = Fight_attacker × roll        (roll ∈ [1−v, 1+v], variance v = tunable)
//! def = Fight_defender × roll
//! connect if atk > def
//! margin      = atk / def − 1        (relative dominance — scale-independent, unbounded)
//! damage_mult = clamp(mult_min + k_margin × margin, mult_min, mult_max)
//! ```
//!
//! and the §7 rules (line 153): `mult_min` MAY be below 1 (a glancing connect),
//! and `mult_max` is set generously so real skill gaps are felt before the clamp.
//! `k_margin` / `mult_min` / `mult_max` / `v` are all tuning values.
//!
//! This module is the **data substrate** (GTW-506 child A): the four tuning
//! leaves + the [`MeleeTuning`] group. The pure §7 functions that consume them
//! ([`opposed_fight`](crate::melee::opposed_fight) /
//! [`melee_damage_mult`](crate::melee::melee_damage_mult)) live in [`crate::melee`];
//! the live melee ACT (adjacency / LOS gate / input / presenter) is GTW-507.
//!
//! ## DESIGN FORK — starting magnitudes (resolution.md §7 line 153)
//!
//! Resolution.md §7 names `k_margin` / `mult_min` / `mult_max` / `v` as tuning but
//! pins no magnitudes. For this substrate the defaults are
//! **defensible-but-arbitrary** starting points, following the
//! [`ReactionTuning`](crate::tuning::ReactionTuning) precedent — balance data,
//! never pinned by a magnitude test. Tests assert only **invariants** (clamp
//! edges, monotonicity, glancing reachable), never these magnitudes. These
//! defaults are forks the user may tune (flagged on GTW-37).

use bevy::prelude::Deref;
use serde::Deserialize;

// ── Tuning leaves ────────────────────────────────────────────────────────────

/// The **margin → damage-mult slope** `k_margin` — how steeply the §7 melee
/// damage multiplier rises with the opposed-Fight margin
/// (`docs/combat/resolution.md` §7 line 150:
/// `damage_mult = clamp(mult_min + k_margin × margin, mult_min, mult_max)`).
///
/// In that formula this is the linear coefficient on `margin = atk/def − 1`: a
/// larger `k_margin` means a given skill gap translates into a bigger damage
/// multiplier before the clamp. Default `1.0` — a defensible-but-arbitrary
/// starting point mirroring the `ReactionCapBase::DEFAULT` precedent (a +50%
/// Fight margin then adds `0.5` to the multiplier); tests assert only the
/// monotonicity / clamp invariants, never this magnitude.
/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner +
/// derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MeleeKMargin(f32);

impl MeleeKMargin {
    /// Build a margin → damage-mult slope from its magnitude (a starting point,
    /// TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `f32` private (house
    /// style) while letting tests and any programmatic tuning edit build a value
    /// without a bare `f32` escaping; shipped values come from the `.ron` via the
    /// derived [`Deserialize`].
    #[must_use]
    pub const fn new(k_margin: f32) -> Self {
        Self(k_margin)
    }
}

impl Default for MeleeKMargin {
    fn default() -> Self {
        // 1.0 — a STARTING POINT (tunable balance data): a +50% Fight margin adds
        // 0.5 to the multiplier before the clamp. Value-agnostic tests only, never
        // a pinned magnitude.
        Self(1.0)
    }
}

/// The **damage-mult clamp floor** `mult_min` — the lowest multiplier a
/// connecting melee hit can apply (`docs/combat/resolution.md` §7 line 150, the
/// `clamp(…, mult_min, mult_max)` lower bound).
///
/// Per resolution.md §7 (line 153) this **may be below 1** — a barely-connecting
/// hit (a tiny positive margin) GLANCES, dealing LESS than its base damage. It is
/// BOTH the clamp floor AND the additive base of `mult_min + k_margin × margin`,
/// so the multiplier at `margin = 0` is exactly `mult_min`. Default `0.5` — a
/// defensible-but-arbitrary starting point that is deliberately `< 1` so the
/// glancing case is reachable; tests assert only that `mult_min < 1` is reachable
/// and the clamp edges hold, never this magnitude. `#[serde(transparent)]` lets
/// it parse a bare RON scalar; private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MeleeMultMin(f32);

impl MeleeMultMin {
    /// Build a damage-mult clamp floor from its magnitude (a starting point, TBD
    /// tuning; resolution.md §7 allows it below 1 for a glancing connect).
    ///
    /// The constructor for the newtype — keeps the inner `f32` private (house
    /// style); shipped values come from the `.ron` via the derived [`Deserialize`].
    #[must_use]
    pub const fn new(mult_min: f32) -> Self {
        Self(mult_min)
    }
}

impl Default for MeleeMultMin {
    fn default() -> Self {
        // 0.5 — a STARTING POINT (tunable balance data), DELIBERATELY below 1 so a
        // barely-connecting hit glances (resolution.md §7 line 153). Value-agnostic
        // tests only, never a pinned magnitude.
        Self(0.5)
    }
}

/// The **damage-mult clamp ceiling** `mult_max` — the highest multiplier a
/// connecting melee hit can apply (`docs/combat/resolution.md` §7 line 150, the
/// `clamp(…, mult_min, mult_max)` upper bound).
///
/// Per resolution.md §7 (line 153) this is set **generously** so real skill gaps
/// are felt before the clamp bites — a ganger who clearly out-fights the target
/// hits much harder, up to this ceiling. Must satisfy `mult_min ≤ mult_max`.
/// Default `3.0` — a defensible-but-arbitrary generous starting point; tests
/// assert only the clamp-edge / ordering invariants, never this magnitude.
/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner +
/// derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MeleeMultMax(f32);

impl MeleeMultMax {
    /// Build a damage-mult clamp ceiling from its magnitude (a generous starting
    /// point per resolution.md §7, TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `f32` private (house
    /// style); shipped values come from the `.ron` via the derived [`Deserialize`].
    #[must_use]
    pub const fn new(mult_max: f32) -> Self {
        Self(mult_max)
    }
}

impl Default for MeleeMultMax {
    fn default() -> Self {
        // 3.0 — a STARTING POINT (tunable balance data), set generously so real
        // skill gaps are felt before the clamp (resolution.md §7 line 153).
        // Value-agnostic tests only, never a pinned magnitude.
        Self(3.0)
    }
}

/// The **opposed-roll variance** `v` — the half-width of the uniform per-side
/// roll in the §7 opposed-Fight check (`docs/combat/resolution.md` §7 lines
/// 146-147: `roll ∈ [1−v, 1+v]`).
///
/// Each side's effective Fight is `Fight × roll` with `roll` drawn uniformly from
/// `[1 − v, 1 + v]`; a larger `v` widens that band, so the same Fight gap connects
/// less reliably (more chance the underdog's roll beats the favourite's). Must
/// satisfy `0.0 ≤ v < 1.0` (a `v ≥ 1` would let a roll reach `0`, collapsing the
/// `atk/def` margin). Default `0.2` (the band `[0.8, 1.2]`) — a
/// defensible-but-arbitrary starting point; tests assert only the
/// distribution-shape invariants, never this magnitude. `#[serde(transparent)]`
/// lets it parse a bare RON scalar; private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FightVariance(f32);

impl FightVariance {
    /// Build an opposed-roll variance from its magnitude (a starting point, TBD
    /// tuning; resolution.md §7 `roll ∈ [1−v, 1+v]`).
    ///
    /// The constructor for the newtype — keeps the inner `f32` private (house
    /// style); shipped values come from the `.ron` via the derived [`Deserialize`].
    #[must_use]
    pub const fn new(variance: f32) -> Self {
        Self(variance)
    }
}

impl Default for FightVariance {
    fn default() -> Self {
        // 0.2 — a STARTING POINT (tunable balance data): the per-side roll band is
        // [0.8, 1.2]. Value-agnostic tests only, never a pinned magnitude.
        Self(0.2)
    }
}

// ── Group struct ─────────────────────────────────────────────────────────────

/// The §7 melee opposed-Fight tuning group — the roll variance and the
/// margin → damage-multiplier curve coefficients (`docs/combat/resolution.md` §7).
///
/// Bundles the four leaves that govern the §7 opposed-Fight resolution: the
/// per-side roll spread ([`FightVariance`]) and the
/// `clamp(mult_min + k_margin × margin, mult_min, mult_max)` damage-multiplier
/// curve ([`MeleeKMargin`] / [`MeleeMultMin`] / [`MeleeMultMax`]). Used as a field
/// of [`crate::tuning::CombatTuning`]; authored in
/// `assets/core_tuning/combat.tuning.ron` under `melee:`.
///
/// The §7 pure functions [`opposed_fight`](crate::melee::opposed_fight) and
/// [`melee_damage_mult`](crate::melee::melee_damage_mult) read these leaves; the
/// live melee ACT is GTW-507.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
pub struct MeleeTuning {
    /// The margin → damage-mult slope `k_margin` of
    /// `clamp(mult_min + k_margin × margin, …)`. Higher = a given skill gap hits
    /// harder before the clamp.
    pub k_margin: MeleeKMargin,
    /// The clamp floor `mult_min` (and the additive base at `margin = 0`). MAY be
    /// below 1 — a glancing connect (resolution.md §7 line 153).
    pub mult_min: MeleeMultMin,
    /// The clamp ceiling `mult_max`. Set generously so real skill gaps are felt
    /// before the clamp (resolution.md §7 line 153).
    pub mult_max: MeleeMultMax,
    /// The per-side roll variance `v`: each roll is uniform in `[1 − v, 1 + v]`
    /// (resolution.md §7 lines 146-147).
    pub variance: FightVariance,
}
