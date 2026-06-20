//! The 8-way grid facing — the [`Direction`] compass and the [`Facing`] component.

use bevy::{
    math::Vec3,
    prelude::{Component, Deref},
};
use serde::Deserialize;

use crate::metric::Cell;

/// One of the eight grid facings a ganger can turn to face.
///
/// The square grid's 8-way compass (cardinals + diagonals): a ganger turns in
/// place between these (combat.md's "turn in place" TU action), and the facing
/// drives the per-facing barrel offset and the faced cell tested for the bracing
/// bonus (resolution.md §1). Eight discrete directions, not a continuous angle —
/// the coarse model reasons in grid steps.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum Direction {
    /// Toward −Y.
    #[default]
    North,
    /// Toward +X, −Y.
    NorthEast,
    /// Toward +X.
    East,
    /// Toward +X, +Y.
    SouthEast,
    /// Toward +Y.
    South,
    /// Toward −X, +Y.
    SouthWest,
    /// Toward −X.
    West,
    /// Toward −X, −Y.
    NorthWest,
}

impl Direction {
    /// This direction's **ground-plane unit step** in sim units — a normalised
    /// `Vec3` pointing the way the facing looks, with `z = 0` (the step lies in the
    /// ground plane; "up" is the separate `+z` axis, `Vec3::Z`).
    ///
    /// The components match the variant's documented orientation (the square grid's
    /// screen-free convention): North is −Y, East is +X, and each diagonal combines
    /// the two cardinals at equal magnitude. Every step is **unit length** — the
    /// cardinals are `±1` on one axis, the diagonals are `±1/√2` on each of two axes
    /// (so a diagonal is a unit vector, not a longer `(±1, ±1)`), making the per-facing
    /// forward offset a true sim-unit displacement (`docs/combat/battle-space.md`
    /// §"Sub-cell precision on the ground plane"). No pixel, no screen-coordinate
    /// convention — these are cubic-voxel sim units.
    #[must_use]
    pub fn forward_step(self) -> Vec3 {
        // The diagonal component: a unit vector's per-axis magnitude on the two
        // axes a diagonal spans (so √(d² + d²) = 1). Derived, not a pixel literal.
        let d = core::f32::consts::FRAC_1_SQRT_2;
        match self {
            Self::North => Vec3::new(0.0, -1.0, 0.0),
            Self::NorthEast => Vec3::new(d, -d, 0.0),
            Self::East => Vec3::new(1.0, 0.0, 0.0),
            Self::SouthEast => Vec3::new(d, d, 0.0),
            Self::South => Vec3::new(0.0, 1.0, 0.0),
            Self::SouthWest => Vec3::new(-d, d, 0.0),
            Self::West => Vec3::new(-1.0, 0.0, 0.0),
            Self::NorthWest => Vec3::new(-d, -d, 0.0),
        }
    }

    /// This direction's **ordinal** on the 8-way ring — `North = 0`, advancing
    /// clockwise through the compass to `NorthWest = 7`.
    ///
    /// The shared ring index the turn helpers ([`steps_to`](Self::steps_to),
    /// [`rotated_toward`](Self::rotated_toward)) reason in: a clockwise step is `+1`
    /// (mod 8), the short-way distance is computed from the ordinal gap. A loop
    /// index into the fixed eight-variant ring (the no-bare-types carve-out for an
    /// index into a collection you own), never a domain quantity stored on a
    /// component.
    const fn ordinal(self) -> u8 {
        match self {
            Self::North => 0,
            Self::NorthEast => 1,
            Self::East => 2,
            Self::SouthEast => 3,
            Self::South => 4,
            Self::SouthWest => 5,
            Self::West => 6,
            Self::NorthWest => 7,
        }
    }

    /// The [`Direction`] at ring ordinal `ord` (taken mod 8) — the inverse of
    /// [`ordinal`](Self::ordinal).
    ///
    /// Total: any `u8` maps to one of the eight variants by wrapping the ordinal
    /// into `0..8`, so the clockwise/counter-clockwise stepping in
    /// [`rotated_toward`](Self::rotated_toward) can never index out of the ring.
    const fn from_ordinal(ord: u8) -> Self {
        match ord % 8 {
            0 => Self::North,
            1 => Self::NorthEast,
            2 => Self::East,
            3 => Self::SouthEast,
            4 => Self::South,
            5 => Self::SouthWest,
            6 => Self::West,
            // 7 (and, after the mod, nothing else) is the only remaining ordinal.
            _ => Self::NorthWest,
        }
    }

    /// The **short-way** count of 45deg steps from this facing to `other` —
    /// `min(d, 8 - d)` where `d` is the ordinal gap. Range `0..=4`.
    ///
    /// `0` iff the two facings are equal, exactly `4` for an opposite facing
    /// (`North`↔`South`), and symmetric (`a.steps_to(b) == b.steps_to(a)`). This is
    /// the number of whole 45deg steps the [`crate::posture::set_facing`] verb must
    /// turn (and pay one [`crate::tuning::TurnTu`] for) to reach `other`. Returns a
    /// bare `u8` step *count* — a loop index / per-step multiplier into the
    /// `TurnTu` leaf (the no-bare-types carve-out for "indices into a collection you
    /// own"), NOT a domain quantity, and never stored on a component. Pure, total,
    /// no panic.
    #[must_use]
    pub const fn steps_to(self, other: Self) -> u8 {
        // The unsigned ordinal gap (both are in 0..8, so this never underflows).
        let d = self.ordinal().abs_diff(other.ordinal());
        // The short way around the ring of eight: never more than half (= 4).
        if d <= 8 - d { d } else { 8 - d }
    }

    /// The 8-way compass [`Direction`] pointing from cell `from` toward cell `to`,
    /// or `None` when the two cells coincide.
    ///
    /// Reads the sign of the `(to - from)` delta on each ground axis (`signum` of
    /// `dx` / `dy`): `dx == 0 && dy == 0` → `None` (no direction toward yourself);
    /// otherwise the sign pair picks one of the eight (e.g. `dx > 0, dy == 0` →
    /// `East`; `dx == 0, dy < 0` → `North` — smaller `y` is North, the `forward_step`
    /// −Y convention; `dx > 0, dy < 0` → `NorthEast`; `dx < 0, dy > 0` → `SouthWest`).
    /// The z axis is irrelevant to a ground facing, so a [`Cell`] (x/y only) is the
    /// input. Pure function of two cells, total, no panic.
    #[must_use]
    pub fn from_cells(from: Cell, to: Cell) -> Option<Self> {
        // signum collapses each axis delta to exactly -1 / 0 / +1 — the 8-way sign pair
        // (recall the −Y convention: smaller y is North). The full sign table is matched
        // on those three literals, so every pair is covered without ordered guards.
        let sx = (to.x - from.x).signum();
        let sy = (to.y - from.y).signum();
        let dir = match (sx, sy) {
            (0, 0) => return None, // coincident cells — no direction toward yourself
            (0, -1) => Self::North,
            (1, -1) => Self::NorthEast,
            (1, 0) => Self::East,
            (1, 1) => Self::SouthEast,
            (0, 1) => Self::South,
            (-1, 1) => Self::SouthWest,
            (-1, 0) => Self::West,
            // The last sign pair: (-1, -1).
            _ => Self::NorthWest,
        };
        Some(dir)
    }

    /// The facing reached after advancing up to `steps` 45deg steps the **short
    /// way** from this facing toward `target`, clamped so it never **overshoots**
    /// `target`.
    ///
    /// Short way: with `cw = (ord(target) + 8 - ord(self)) % 8` (clockwise gap) and
    /// `ccw = (ord(self) + 8 - ord(target)) % 8` (counter-clockwise gap), rotate
    /// clockwise (`+1` per step) when `cw <= ccw`, else counter-clockwise (`-1`), for
    /// `n = min(steps, min(cw, ccw))` steps. The opposite-facing tie (`cw == ccw ==
    /// 4`) breaks **clockwise**. So `d.rotated_toward(d, _)` and
    /// `d.rotated_toward(t, 0)` return `d` (`self`), and any `steps >=
    /// self.steps_to(target)` returns `target` (the clamp). This is what lets a
    /// PARTIAL turn land on an intermediate facing when the TU pool runs out before
    /// the full rotation. Pure, total, no panic.
    #[must_use]
    pub const fn rotated_toward(self, target: Self, steps: u8) -> Self {
        let from = self.ordinal();
        let to = target.ordinal();
        // The two ways around the ring (both in 0..8). cw + ccw == 8 unless equal.
        let cw = (to + 8 - from) % 8;
        let ccw = (from + 8 - to) % 8;
        // The short way's length; the tie (cw == ccw == 4) prefers clockwise.
        let short = if cw <= ccw { cw } else { ccw };
        // Clamp the requested steps so the turn never overshoots `target`.
        let n = if steps < short { steps } else { short };
        if cw <= ccw {
            Self::from_ordinal(from + n) // clockwise: +1 per step (mod 8 in from_ordinal)
        } else {
            // counter-clockwise: -1 per step, kept non-negative by adding a full ring.
            Self::from_ordinal(from + 8 - n)
        }
    }
}

/// A ganger's facing — which of the eight grid [`Direction`]s it currently faces.
///
/// A distinct component so a turn/LOS system can query `&Facing` alone. Defaults
/// to [`Direction::North`] (a structural spawn default — the canonical "facing up
/// the grid" orientation, not a balance value). `#[serde(transparent)]` lets an
/// authored facing parse as the bare [`Direction`] variant.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Facing(Direction);

impl Facing {
    /// Build a facing from the [`Direction`] the ganger faces.
    ///
    /// The public constructor (private inner + constructor, the crate's newtype
    /// house style) so the situation→entities setup (E1.8 / GTW-158) can build a
    /// `Facing` from an authored direction without reaching the private field.
    #[must_use]
    pub const fn new(direction: Direction) -> Self {
        Self(direction)
    }
}
