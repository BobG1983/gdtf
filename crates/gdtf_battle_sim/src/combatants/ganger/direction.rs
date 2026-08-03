//! Eight-way facing and ring rotation.

use bevy::{
    math::Vec3,
    prelude::{Component, Deref},
};
use serde::Deserialize;

use crate::metric::Cell;

/// Unit vector one step along a direction (sim space).
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ForwardStep(Vec3);

impl ForwardStep {
    /// Wrap a step vector.
    #[must_use]
    pub const fn new(step: Vec3) -> Self {
        Self(step)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct RingOrdinal(u8);

impl RingOrdinal {
    const fn new(ordinal: u8) -> Self {
        Self(ordinal)
    }
}

/// Number of 45° ring steps between two directions.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RingSteps(u8);

impl RingSteps {
    /// Wrap a step count.
    #[must_use]
    pub const fn new(steps: u8) -> Self {
        Self(steps)
    }
}

/// Compass facing on the grid.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum Direction {
    /// North (-Y).
    #[default]
    North,
    /// Northeast.
    NorthEast,
    /// East (+X).
    East,
    /// Southeast.
    SouthEast,
    /// South (+Y).
    South,
    /// Southwest.
    SouthWest,
    /// West (-X).
    West,
    /// Northwest.
    NorthWest,
}

impl Direction {
    /// Continuous forward step in sim space.
    #[must_use]
    pub fn forward_step(self) -> ForwardStep {
        let d = core::f32::consts::FRAC_1_SQRT_2;
        ForwardStep::new(match self {
            Self::North => Vec3::new(0.0, -1.0, 0.0),
            Self::NorthEast => Vec3::new(d, -d, 0.0),
            Self::East => Vec3::new(1.0, 0.0, 0.0),
            Self::SouthEast => Vec3::new(d, d, 0.0),
            Self::South => Vec3::new(0.0, 1.0, 0.0),
            Self::SouthWest => Vec3::new(-d, d, 0.0),
            Self::West => Vec3::new(-1.0, 0.0, 0.0),
            Self::NorthWest => Vec3::new(-d, -d, 0.0),
        })
    }

    /// Integer cell delta for one step.
    #[must_use]
    pub const fn cell_step(self) -> Cell {
        match self {
            Self::North => Cell::new(0, -1),
            Self::NorthEast => Cell::new(1, -1),
            Self::East => Cell::new(1, 0),
            Self::SouthEast => Cell::new(1, 1),
            Self::South => Cell::new(0, 1),
            Self::SouthWest => Cell::new(-1, 1),
            Self::West => Cell::new(-1, 0),
            Self::NorthWest => Cell::new(-1, -1),
        }
    }

    const fn ordinal(self) -> RingOrdinal {
        match self {
            Self::North => RingOrdinal::new(0),
            Self::NorthEast => RingOrdinal::new(1),
            Self::East => RingOrdinal::new(2),
            Self::SouthEast => RingOrdinal::new(3),
            Self::South => RingOrdinal::new(4),
            Self::SouthWest => RingOrdinal::new(5),
            Self::West => RingOrdinal::new(6),
            Self::NorthWest => RingOrdinal::new(7),
        }
    }

    const fn from_ordinal(ord: RingOrdinal) -> Self {
        match ord.0 % 8 {
            0 => Self::North,
            1 => Self::NorthEast,
            2 => Self::East,
            3 => Self::SouthEast,
            4 => Self::South,
            5 => Self::SouthWest,
            6 => Self::West,
            _ => Self::NorthWest,
        }
    }

    /// Shortest ring distance to `other`.
    #[must_use]
    pub const fn steps_to(self, other: Self) -> RingSteps {
        let d = self.ordinal().0.abs_diff(other.ordinal().0);
        RingSteps::new(if d <= 8 - d { d } else { 8 - d })
    }

    /// Direction from one cell toward another (`None` if same cell).
    #[must_use]
    pub fn from_cells(from: Cell, to: Cell) -> Option<Self> {
        let sx = (to.x - from.x).signum();
        let sy = (to.y - from.y).signum();
        let dir = match (sx, sy) {
            (0, 0) => return None,
            (0, -1) => Self::North,
            (1, -1) => Self::NorthEast,
            (1, 0) => Self::East,
            (1, 1) => Self::SouthEast,
            (0, 1) => Self::South,
            (-1, 1) => Self::SouthWest,
            (-1, 0) => Self::West,
            _ => Self::NorthWest,
        };
        Some(dir)
    }

    /// Rotate up to `steps` toward `target` along the short arc.
    #[must_use]
    pub const fn rotated_toward(self, target: Self, steps: RingSteps) -> Self {
        let from = self.ordinal().0;
        let to = target.ordinal().0;
        let cw = (to + 8 - from) % 8;
        let ccw = (from + 8 - to) % 8;
        let short = if cw <= ccw { cw } else { ccw };
        let n = if steps.0 < short { steps.0 } else { short };
        if cw <= ccw {
            Self::from_ordinal(RingOrdinal::new(from + n))
        } else {
            Self::from_ordinal(RingOrdinal::new(from + 8 - n))
        }
    }
}

/// Current facing direction component.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Facing(Direction);

impl Facing {
    /// Wrap a direction.
    #[must_use]
    pub const fn new(direction: Direction) -> Self {
        Self(direction)
    }
}
