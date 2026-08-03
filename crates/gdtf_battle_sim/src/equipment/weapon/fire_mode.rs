use std::fmt::Display;

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModeConeMult(f32);

impl ModeConeMult {
    #[must_use]
    pub const fn new(mult: f32) -> Self {
        Self(mult)
    }
}

/// `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModeTuPercent(f32);

impl ModeTuPercent {
    #[must_use]
    pub const fn new(percent: f32) -> Self {
        Self(percent)
    }
}

/// + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModeShots(u16);

impl ModeShots {
    #[must_use]
    pub const fn new(shots: u16) -> Self {
        Self(shots)
    }
}

/// Fire-mode selector kind held on a [`FireMode`] entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModeKind {
    Single,
    Burst,
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

/// Blast radius in cells. Private inner + [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlastRadius(u8);

impl BlastRadius {
    #[must_use]
    pub const fn new(radius: u8) -> Self {
        Self(radius)
    }
}

/// AOE range in cells. `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AoeRange(u8);

impl AoeRange {
    #[must_use]
    pub const fn new(range: u8) -> Self {
        Self(range)
    }
}

/// Cone half-angle in degrees. [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConeHalfAngle(f32);

impl ConeHalfAngle {
    #[must_use]
    pub const fn new(degrees: f32) -> Self {
        Self(degrees)
    }
}

/// Hit geometry for a fire mode. [`Single`](Self::Single) is the default.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum HitType {
    #[default]
    Single,
    Blast {
        radius: BlastRadius,
    },
    Cone {
        range: AoeRange,
        angle: ConeHalfAngle,
    },
    Line {
        range: AoeRange,
    },
}

/// Spec for one entry in a weapon's fire-mode list.
///
/// `hit_type` defaults to [`HitType::Single`] so existing weapon RON stays valid.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FireModeSpec {
    pub kind:       ModeKind,
    pub cone_mult:  ModeConeMult,
    pub tu_percent: ModeTuPercent,
    pub shots:      ModeShots,
    /// Defaults to [`HitType::Single`] when omitted in RON.
    #[serde(default)]
    pub hit_type: HitType,
}

impl FireModeSpec {
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

/// Weapon fire-mode list (component). Deserializes from a bare RON list of specs.
#[derive(Component, Deref, Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FireMode(Vec<FireModeSpec>);

impl FireMode {
    #[must_use]
    pub const fn new(modes: Vec<FireModeSpec>) -> Self {
        Self(modes)
    }

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
