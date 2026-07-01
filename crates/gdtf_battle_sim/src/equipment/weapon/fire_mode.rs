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

/// A **blast radius** — the Chebyshev cell distance a [`HitType::Blast`] template
/// reaches out from its impact cell (`docs/combat/combat.md`'s blast-radii note —
/// "blast radii ... assume square tiles"). Radius `0` = the impact cell only; radius
/// `1` = the impact cell + its Moore-8 ring; and so on. The template geometry is
/// defined in [`aoe`](crate::shot_pipeline::aoe), not in `docs/`.
///
/// A weapon NUMBER (per-mode, on the [`FireModeSpec`]), a small non-negative cell
/// count. Private inner + derived [`Deref`]; `#[serde(transparent)]` so a weapon's
/// authored RON writes the bare integer (`Blast(radius: 2)`).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlastRadius(u8);

impl BlastRadius {
    /// Build a blast radius from its Chebyshev cell count.
    #[must_use]
    pub const fn new(radius: u8) -> Self {
        Self(radius)
    }
}

/// An `AoE` **range** — the cell depth a [`HitType::Cone`] wedge or a [`HitType::Line`]
/// runs from its origin (`docs/combat/combat.md`'s blast-radii note; the wedge/beam
/// geometry is defined in [`aoe`](crate::shot_pipeline::aoe)). Range `0` = no cells
/// beyond the impact; range `n` = up to `n` cells deep along the shape.
///
/// A weapon NUMBER (per-mode), a small non-negative cell count. Distinct from
/// [`BlastRadius`] (no-bare-types rule 3 — a directed range is a different concept
/// than an omnidirectional radius). Private inner + derived [`Deref`];
/// `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AoeRange(u8);

impl AoeRange {
    /// Build an `AoE` range from its cell depth.
    #[must_use]
    pub const fn new(range: u8) -> Self {
        Self(range)
    }
}

/// A **cone half-angle** — how wide a [`HitType::Cone`] wedge opens, in degrees off the
/// firing direction on each side (the wedge geometry is defined in
/// [`aoe`](crate::shot_pipeline::aoe), not in `docs/`). A cell is inside the wedge when
/// the angle between (cell − origin) and the fire direction is ≤ this half-angle.
///
/// A weapon NUMBER (per-mode), an angular magnitude in DEGREES. Private inner + derived
/// [`Deref`]; `#[serde(transparent)]` so a weapon's RON writes the bare float
/// (`Cone(range: 3, angle: 30.0)`). Angular / dimensionless — **zero pixels**.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConeHalfAngle(f32);

impl ConeHalfAngle {
    /// Build a cone half-angle from its magnitude in degrees.
    #[must_use]
    pub const fn new(degrees: f32) -> Self {
        Self(degrees)
    }
}

/// A fire mode's **hit type** — the `AoE` TEMPLATE a shot in this mode applies at its
/// impact cell (`docs/combat/combat.md`'s blast-radii note; the template geometry is
/// defined in [`aoe`](crate::shot_pipeline::aoe); GTW-41 CORE, GTW-541). A direct
/// single-target shot, an omnidirectional [`Blast`](Self::Blast)
/// disc, a directed [`Cone`](Self::Cone) wedge, or a [`Line`](Self::Line) beam.
///
/// A named domain enum (no-bare-types: a hit type is a domain value, not a bare `u8`),
/// a FIELD value on a [`FireModeSpec`] (NOT a `#[derive(Component)]` — the [`FireMode`]
/// selector that holds the specs is the component). Its fields are the `AoE` shape
/// newtypes ([`BlastRadius`] / [`AoeRange`] / [`ConeHalfAngle`]). **Distinct from
/// [`DamageType::Blast`](super::DamageType) — that is a damage FLAVOUR (the matchup-wheel
/// node); this is a hit SHAPE.**
///
/// [`Single`](Self::Single) is the DEFAULT (`#[serde(default)]` on the
/// [`FireModeSpec`] field) so every existing weapon `.ron` — which never authors a
/// `hit_type` — deserializes byte-identically to a direct single-target shot, and the
/// live fire path takes the unchanged single-target branch for it (the IDENTITY
/// property, GTW-541 AC). `Copy`/`Eq` where the fields allow, so [`FireModeSpec`] stays
/// `Copy` and the message payload owns it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HitType {
    /// A **direct single-target** shot — the unchanged pre-GTW-541 path (the impact
    /// cell's occupant, if any, is the sole struck entity). The DEFAULT.
    Single,
    /// An **omnidirectional blast** — every cell within `radius` (Chebyshev, same
    /// storey) of the impact cell is affected. Radius `0` = the impact cell only.
    Blast {
        /// The Chebyshev cell reach of the blast disc out from the impact cell.
        radius: BlastRadius,
    },
    /// A directed **cone** — a wedge opening from the shooter toward the impact,
    /// `range` cells deep and `angle` degrees wide on each side of the fire direction.
    Cone {
        /// The cell depth the wedge runs.
        range: AoeRange,
        /// The half-angle (degrees off the fire direction) the wedge spans each side.
        angle: ConeHalfAngle,
    },
    /// A **line / beam** — the cells along the shot line from the impact cell, `range`
    /// cells deep in the fire direction.
    Line {
        /// The cell depth the line runs from the impact cell.
        range: AoeRange,
    },
}

impl Default for HitType {
    /// The default hit type is [`Single`](Self::Single) — a direct single-target shot,
    /// so an omitted `hit_type:` field leaves a weapon firing exactly as before GTW-541.
    fn default() -> Self {
        Self::Single
    }
}

/// One fire mode's per-mode numbers — its kind, cone multiplier, TU%, shot count, and
/// `AoE` hit type.
///
/// The selector term carrier of `θ_cone` (resolution.md §1a) plus the mode's kind,
/// TU cost, round count, and the `AoE` [`HitType`] template (GTW-541). A named struct
/// (not a bare tuple) so each per-mode number keeps its [`FireMode`] meaning; every
/// field is a weapon NUMBER newtype or a closed domain enum. The human-facing label
/// comes from [`ModeKind`]'s [`Display`] (`kind.to_string()`), not a stored string.
///
/// Every field is `Copy`, so the spec is `Copy` (it regained the derive once the
/// `String` mode name was dropped — GTW-260; it was only `Clone`-not-`Copy` because
/// of the old owned name). The [`hit_type`](FireModeSpec::hit_type) is
/// `#[serde(default)]` = [`HitType::Single`], so every existing weapon `.ron` (which
/// authors no `hit_type`) parses byte-identically (GTW-541 IDENTITY).
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
    /// The `AoE` template a shot in this mode applies at its impact cell (GTW-541).
    /// `#[serde(default)]` = [`HitType::Single`], so an omitted `hit_type:` field keeps
    /// the mode a direct single-target shot — every existing weapon `.ron` is untouched.
    #[serde(default)]
    pub hit_type:   HitType,
}

impl FireModeSpec {
    /// Build one fire mode's spec from its kind and its per-mode numbers, defaulting the
    /// `AoE` [`HitType`] to [`HitType::Single`] (the direct single-target shot).
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
            hit_type: HitType::Single,
        }
    }

    /// Build one fire mode's spec with an explicit `AoE` [`HitType`] template — the
    /// constructor a blast / cone / line mode uses (the direct single-target modes take
    /// [`new`](Self::new), which defaults `hit_type` to [`HitType::Single`]).
    #[must_use]
    pub const fn with_hit_type(
        kind: ModeKind,
        cone_mult: ModeConeMult,
        tu_percent: ModeTuPercent,
        shots: ModeShots,
        hit_type: HitType,
    ) -> Self {
        Self {
            kind,
            cone_mult,
            tu_percent,
            shots,
            hit_type,
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
