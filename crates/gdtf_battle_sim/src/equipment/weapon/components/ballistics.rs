//! The §1a/§1b cone + stability terms — [`BaseSpread`], [`Accuracy`], [`Kickback`],
//! and the braced-by-design [`Stable`] tag.

use bevy::prelude::{Component, Deref};
use serde::Deserialize;

/// A weapon's **base spread** — the intrinsic angular dispersion before the
/// situational multipliers, the `base_spread` term of `θ_cone` (resolution.md
/// §1a). The widest the cone can throw from this weapon's mechanics alone, in the
/// sim's angular unit (radians; the cone math is angle-only, no pixel).
///
/// A weapon NUMBER (lives on the weapon, not in tuning). Private inner + derived
/// [`Deref`]; `#[serde(transparent)]` parses a bare RON scalar. A
/// `#[derive(Component)]` so it lives as a sibling component on the armed entity
/// (GTW-200).
/// `Default` (`BaseSpread(0.0)`) is a **spawn-seed sentinel only** — the
/// `bsn!`-scene spawn path's `get_or_insert_template` seeds the component slot
/// via `Default` before the authored `BaseSpread::new(..)` overwrites it
/// (GTW-322). It is NOT a valid authored weapon stat.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize, Default)]
#[serde(transparent)]
pub struct BaseSpread(f32);

impl BaseSpread {
    /// Build a base-spread value from its magnitude (radians).
    #[must_use]
    pub const fn new(radians: f32) -> Self {
        Self(radians)
    }
}

/// A weapon's **accuracy** — the weapon term of the concentration exponent
/// `p = concentration_p(Shooting, weapon.accuracy)` (resolution.md §1b): higher
/// accuracy clusters the in-cone draw toward dead-center. **Can exceed 1.0**
/// (resolution.md §1b: "The weapon term can exceed 1.0").
///
/// A weapon NUMBER (not a tuning coefficient). It sets how *likely* a shot stays
/// near center — independent of how *wide* the cone can throw (the two levers of
/// §1b). Private inner + derived [`Deref`]; `#[serde(transparent)]`. A
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
/// `Default` (`Accuracy(0.0)`) is a **spawn-seed sentinel only** — the `bsn!`
/// spawn path seeds the slot via `Default` before `Accuracy::new(..)` overwrites
/// it (GTW-322). It is NOT a valid authored weapon stat.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize, Default)]
#[serde(transparent)]
pub struct Accuracy(f32);

impl Accuracy {
    /// Build an accuracy value from its magnitude (dimensionless; may exceed 1.0).
    #[must_use]
    pub const fn new(accuracy: f32) -> Self {
        Self(accuracy)
    }
}

/// A weapon's **kickback** — the per-round recoil it adds, the `kickback` term of
/// the recoil factor `recoil = 1 + prior_shots × kickback × recoil_growth` (resolution.md §1a):
/// each round in a burst widens the cone for the next. A sloppy weapon sprays on
/// auto; a tight one stays usable (resolution.md §1a "Scaling recoil").
///
/// A weapon NUMBER. Private inner + derived [`Deref`]; `#[serde(transparent)]`. A
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
/// `Default` (`Kickback(0.0)`) is a **spawn-seed sentinel only** — the `bsn!`
/// spawn path seeds the slot via `Default` before `Kickback::new(..)` overwrites
/// it (GTW-322). It is NOT a valid authored weapon stat.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize, Default)]
#[serde(transparent)]
pub struct Kickback(f32);

impl Kickback {
    /// Build a kickback value from its per-round recoil magnitude.
    #[must_use]
    pub const fn new(kickback: f32) -> Self {
        Self(kickback)
    }
}

/// A weapon's **`stable` tag** — whether the weapon is braced-by-design
/// (bipod-mounted / a heavy, inherently-steady piece). A `stable` weapon engages
/// the §1a brace / cover-stability bonus **unconditionally** — the brace
/// contribution applies regardless of the faced cell's cover height or the
/// ganger's stance (it bypasses the normal §1a brace min-height gate, so a stable
/// weapon is as steady as a properly-braced one even facing an empty/unsuitable
/// cell). This is the model the user corrected to: weapons carry **no** intrinsic
/// stability *points* — only this boolean tag.
///
/// A weapon NUMBER (a boolean flag, lives on the weapon, not in tuning). A named
/// newtype (no-bare-types: a `bool` carrying domain meaning is wrapped). Private
/// inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON `true` /
/// `false`. A `#[derive(Component)]` (GTW-200) — a sibling component on the armed
/// entity.
/// `Default` (`Stable(false)`) is a **spawn-seed sentinel only** — the `bsn!`
/// spawn path seeds the slot via `Default` before `Stable::new(..)` overwrites it
/// (GTW-322). It is NOT a valid authored weapon tag (though `false` happens to
/// coincide with "not braced-by-design", it is overwritten regardless).
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
#[serde(transparent)]
pub struct Stable(bool);

impl Stable {
    /// Build a `stable` tag from its boolean value (`true` = braced-by-design,
    /// unconditional brace).
    #[must_use]
    pub const fn new(stable: bool) -> Self {
        Self(stable)
    }
}
