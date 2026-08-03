//! Fire-mode list and hit geometry (single, blast, cone, line).

use std::fmt::Display;

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// Multiplier applied to base cone spread for this mode.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModeConeMult(f32);

impl ModeConeMult {
    /// Wrap a multiplier.
    #[must_use]
    pub const fn new(mult: f32) -> Self {
        Self(mult)
    }
}

/// Fraction of max TU spent to fire this mode.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModeTuPercent(f32);

impl ModeTuPercent {
    /// Wrap a TU fraction.
    #[must_use]
    pub const fn new(percent: f32) -> Self {
        Self(percent)
    }
}

/// Number of shots in the mode (burst size).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModeShots(u16);

impl ModeShots {
    /// Wrap a shot count.
    #[must_use]
    pub const fn new(shots: u16) -> Self {
        Self(shots)
    }
}

/// Fire-mode selector kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModeKind {
    /// One shot.
    Single,
    /// Short burst.
    Burst,
    /// Full auto.
    Full,
}

impl Display for ModeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Single => "single",
            Self::Burst => "burst",
            Self::Full => "full-auto",
        };
        f.write_str(label)
    }
}

/// Blast radius in cells.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlastRadius(u8);

impl BlastRadius {
    /// Wrap a radius.
    #[must_use]
    pub const fn new(radius: u8) -> Self {
        Self(radius)
    }
}

/// AOE range in cells.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AoeRange(u8);

impl AoeRange {
    /// Wrap a range.
    #[must_use]
    pub const fn new(range: u8) -> Self {
        Self(range)
    }
}

/// Cone half-angle in degrees.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConeHalfAngle(f32);

impl ConeHalfAngle {
    /// Wrap an angle.
    #[must_use]
    pub const fn new(degrees: f32) -> Self {
        Self(degrees)
    }
}

/// Hit geometry for a fire mode.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum HitType {
    /// Single target.
    #[default]
    Single,
    /// Circular blast.
    Blast {
        /// Radius in cells.
        radius: BlastRadius,
    },
    /// Cone AOE.
    Cone {
        /// Range in cells.
        range: AoeRange,
        /// Half-angle in degrees.
        angle: ConeHalfAngle,
    },
    /// Line AOE.
    Line {
        /// Range in cells.
        range: AoeRange,
    },
}

/// One entry in a weapon's fire-mode list.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FireModeSpec {
    /// Single / burst / full.
    pub kind: ModeKind,
    /// Cone mult.
    pub cone_mult: ModeConeMult,
    /// TU cost as fraction of max.
    pub tu_percent: ModeTuPercent,
    /// Shots fired.
    pub shots: ModeShots,
    /// Hit geometry (defaults to single).
    #[serde(default)]
    pub hit_type: HitType,
}

impl FireModeSpec {
    /// Spec with single-target hits.
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

    /// Spec with explicit hit geometry.
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

/// Weapon fire-mode list component.
#[derive(Component, Deref, Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FireMode(Vec<FireModeSpec>);

impl FireMode {
    /// Wrap a list of modes.
    #[must_use]
    pub const fn new(modes: Vec<FireModeSpec>) -> Self {
        Self(modes)
    }

    /// Prefer the single-shot mode, else first mode, else a safe default.
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
