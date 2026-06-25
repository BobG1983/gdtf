//! The **fire-mode model** — the per-mode numbers ([`ModeConeMult`] /
//! [`ModeTuPercent`] / [`ModeShots`]), the closed [`ModeKind`] + its [`Display`]
//! label, the per-mode [`FireModeSpec`], and the [`FireMode`] selector that lists
//! a weapon's offered modes (GTW-200 / GTW-260).

use std::fmt::Display;

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// A per-mode **cone multiplier** — the selector term of `θ_cone` (the `firemode`
/// factor, resolution.md §1a): single ≈ 1, full-auto ≥ 1 (inherently sloppier).
/// A multiplier on the cone's angular size for that fire mode.
///
/// A weapon NUMBER (per-mode, on the [`FireMode`]). Private inner + derived
/// [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModeConeMult(f32);

impl ModeConeMult {
    /// Build a per-mode cone multiplier from its magnitude.
    #[must_use]
    pub const fn new(mult: f32) -> Self {
        Self(mult)
    }
}

/// A per-mode **TU percentage** — the fraction of the shooter's TU pool a shot in
/// this mode costs (resolution.md §1: per-`FireMode` TU%). A dimensionless
/// fraction of the per-shot TU charge (the aim-mode ×1.5 premium, a tuning
/// coefficient, stacks on top — resolution.md §1a).
///
/// A weapon NUMBER (per-mode). Private inner + derived [`Deref`];
/// `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModeTuPercent(f32);

impl ModeTuPercent {
    /// Build a per-mode TU percentage from its magnitude (a fraction of the TU
    /// pool).
    #[must_use]
    pub const fn new(percent: f32) -> Self {
        Self(percent)
    }
}

/// A per-mode **shot count** — how many rounds the mode fires per shot action
/// (resolution.md §1: per-`FireMode` shots; single = 1, a burst > 1, full-auto
/// more), the burst loop `fire()` runs (resolution.md §"What's pure math vs sim").
/// Each successive round adds the weapon's [`Kickback`](super::Kickback) to the
/// recoil factor.
///
/// A weapon NUMBER (per-mode), a small non-negative count (`u16`). Private inner
/// + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModeShots(u16);

impl ModeShots {
    /// Build a per-mode shot count from its round-per-action number.
    #[must_use]
    pub const fn new(shots: u16) -> Self {
        Self(shots)
    }
}

/// A fire mode's **kind** — the closed set of mechanics a mode can be: a single
/// shot, a burst, or full-auto (resolution.md §1: the selector offers "single /
/// burst / full-auto" modes). This is the model the user corrected to: a mode is
/// identified by a **closed enum** (`Single` / `Burst` / `Full`), not by a stored
/// human-facing name string — the label is derived from this kind via [`Display`].
///
/// A named domain enum (no-bare-types: a fire-mode kind is a domain value, not a
/// bare `u8`/label string). It is a FIELD value on a [`FireModeSpec`], NOT a
/// `#[derive(Component)]` — the [`FireMode`] selector that holds the specs is the
/// component. `Deserialize` so a weapon's authored RON names its mode kind by
/// variant; `Serialize` for the RON round-trip; `Copy`/`Eq`/`Hash` so a
/// [`FireModeSpec`] is `Copy` and the cycle compares mode identity by kind.
///
/// [`Display`] yields the canonical human labels (`"single"` / `"burst"` /
/// `"full-auto"`) — the picker (GTW-254) shows these, derived from the kind rather
/// than stored as a per-mode `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModeKind {
    /// A single aimed/snap shot — one round per shot action (the baseline mode).
    Single,
    /// A short burst — a few rounds per shot action, wider than single.
    Burst,
    /// Full-auto — the most rounds per shot action, the sloppiest spread.
    Full,
}

impl Display for ModeKind {
    /// The canonical human-facing mode LABEL the picker shows — `"single"` /
    /// `"burst"` / `"full-auto"` — derived from the kind (no stored name string).
    /// These preserve the prior per-mode label strings (the deleted GTW-256
    /// name-string newtype).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Single => "single",
            Self::Burst => "burst",
            Self::Full => "full-auto",
        };
        f.write_str(label)
    }
}

/// One fire mode's per-mode numbers — its kind, cone multiplier, TU%, and shot count.
///
/// The selector term carrier of `θ_cone` (resolution.md §1a) plus the mode's kind,
/// TU cost, and round count. A named struct (not a bare tuple) so each per-mode
/// number keeps its [`FireMode`] meaning; every field is a weapon NUMBER newtype or
/// the closed [`ModeKind`]. The human-facing label comes from
/// [`ModeKind`]'s [`Display`] (`kind.to_string()`), not a stored string.
///
/// Every field is `Copy`, so the spec is `Copy` (it regained the derive once the
/// `String` mode name was dropped — GTW-260; it was only `Clone`-not-`Copy` because
/// of the old owned name).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FireModeSpec {
    /// Which mode this is — its closed kind (`Single` / `Burst` / `Full`); the
    /// human-facing label is `kind.to_string()`.
    pub kind:       ModeKind,
    /// The selector cone multiplier for this mode (single ≈ 1, full-auto ≥ 1).
    pub cone_mult:  ModeConeMult,
    /// The fraction of the TU pool a shot in this mode costs.
    pub tu_percent: ModeTuPercent,
    /// How many rounds this mode fires per shot action.
    pub shots:      ModeShots,
}

impl FireModeSpec {
    /// Build one fire mode's spec from its kind and its three per-mode numbers.
    #[must_use]
    pub const fn new(
        kind: ModeKind,
        cone_mult: ModeConeMult,
        tu_percent: ModeTuPercent,
        shots: ModeShots,
    ) -> Self {
        Self {
            kind,
            cone_mult,
            tu_percent,
            shots,
        }
    }
}

/// A weapon's **fire-mode selector** — the LIST of modes a weapon offers, each a
/// [`FireModeSpec`] paired with its closed [`ModeKind`] (resolution.md §1: the
/// selector is authored per weapon). A weapon offers **any subset of `{Single,
/// Burst, Full}` in authored order** (GTW-260 broadened the old three fixed ladders
/// — single / single+burst / single+burst+full-auto — to an arbitrary ordered list;
/// the docs-sync recording this is GTW-258).
///
/// A named newtype over `Vec<`[`FireModeSpec`]`>` (no-bare-types: the selector is a
/// domain value; the inner `Vec` is the collection-of-domain-values carve-out). The
/// private inner + derived [`Deref`] gives slice access (`.iter()` / `.len()` /
/// `.get()`); `#[serde(transparent)]` so it deserializes from a **bare RON list** of
/// mode entries (`fire_mode: [ (kind: Single, …), … ]`). `Clone`-not-`Copy` (it
/// holds a `Vec`). A `#[derive(Component)]` (GTW-200) — the selector lives as a
/// sibling component on the armed entity (its per-mode [`FireModeSpec`] sub-values
/// ride inside it, not as separate components).
///
/// **Invariant:** a well-authored weapon lists at least one mode, with `Single`
/// first. The code is DEFENSIVE if that is violated — every read has a total
/// fallback and never panics (see [`FireMode::single`]).
/// `Default` (`FireMode(Vec::new())`, the empty selector) is a **spawn-seed
/// sentinel only** — the `bsn!` spawn path seeds the slot via `Default` before
/// `FireMode::new(..)` overwrites it (GTW-322). An empty selector is NOT a valid
/// authored weapon; every read has a total fallback (see [`FireMode::single`]),
/// but the spawn path always overwrites the sentinel with the authored list.
#[derive(Component, Deref, Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FireMode(Vec<FireModeSpec>);

impl FireMode {
    /// Build a fire-mode selector from its authored list of modes.
    #[must_use]
    pub const fn new(modes: Vec<FireModeSpec>) -> Self {
        Self(modes)
    }

    /// The **single-shot spec** — the mode whose [`ModeKind`] is
    /// [`ModeKind::Single`]; else the FIRST authored mode; else a structural
    /// single-shot default (`Single`, cone ×1.0, 0% TU, 1 shot). Returns BY VALUE
    /// ([`FireModeSpec`] is `Copy` again). The fallback chain is TOTAL — NO `unwrap`
    /// / `panic` even for an empty or `Single`-less (mis-authored) selector.
    #[must_use]
    pub fn single(&self) -> FireModeSpec {
        if let Some(single) = self.0.iter().find(|spec| spec.kind == ModeKind::Single) {
            *single
        } else if let Some(first) = self.0.first() {
            *first
        } else {
            FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.0),
                ModeShots::new(1),
            )
        }
    }
}
