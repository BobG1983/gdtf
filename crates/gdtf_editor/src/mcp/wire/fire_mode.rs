//! The fire-mode row an attachment's `GainFireMode` effect carries, on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::weapon::{
    AoeRange, BlastRadius, ConeHalfAngle, FireModeSpec, HitType, ModeConeMult, ModeKind, ModeShots,
    ModeTuPercent,
};
use serde::{Deserialize, Serialize};

/// Which selector a fire mode is fired on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum ModeKindNet {
    /// One shot.
    Single,
    /// Short burst.
    Burst,
    /// Full auto.
    Full,
}

impl ModeKindNet {
    /// Mirror the sim's own mode kind.
    pub(in crate::mcp) const fn from_kind(kind: ModeKind) -> Self {
        match kind {
            ModeKind::Single => Self::Single,
            ModeKind::Burst => Self::Burst,
            ModeKind::Full => Self::Full,
        }
    }

    /// Read a client's mode kind back as the sim's own.
    pub(in crate::mcp) const fn to_kind(self) -> ModeKind {
        match self {
            Self::Single => ModeKind::Single,
            Self::Burst => ModeKind::Burst,
            Self::Full => ModeKind::Full,
        }
    }
}

/// The multiplier a mode applies to base cone spread.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct ModeConeMultNet(f32);

impl ModeConeMultNet {
    /// Wrap a cone multiplier.
    pub(in crate::mcp) const fn new(mult: f32) -> Self {
        Self(mult)
    }
}

/// The fraction of max TU a mode spends.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct ModeTuPercentNet(f32);

impl ModeTuPercentNet {
    /// Wrap a TU fraction.
    pub(in crate::mcp) const fn new(percent: f32) -> Self {
        Self(percent)
    }
}

/// How many shots a mode fires.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct ModeShotsNet(u16);

impl ModeShotsNet {
    /// Wrap a shot count.
    pub(in crate::mcp) const fn new(shots: u16) -> Self {
        Self(shots)
    }
}

/// A blast's radius in cells.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct BlastRadiusNet(u8);

impl BlastRadiusNet {
    /// Wrap a blast radius.
    pub(in crate::mcp) const fn new(radius: u8) -> Self {
        Self(radius)
    }
}

/// A cone's or line's range in cells.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct AoeRangeNet(u8);

impl AoeRangeNet {
    /// Wrap an AOE range.
    pub(in crate::mcp) const fn new(range: u8) -> Self {
        Self(range)
    }
}

/// A cone's half-angle in degrees.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct ConeHalfAngleNet(f32);

impl ConeHalfAngleNet {
    /// Wrap a half-angle in degrees.
    pub(in crate::mcp) const fn new(degrees: f32) -> Self {
        Self(degrees)
    }
}

/// The geometry a fire mode's shots land on.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(in crate::mcp) enum HitTypeNet {
    /// One target.
    Single,
    /// A circular blast.
    Blast {
        /// Radius in cells.
        radius: BlastRadiusNet,
    },
    /// A cone.
    Cone {
        /// Range in cells.
        range: AoeRangeNet,
        /// Half-angle in degrees.
        angle: ConeHalfAngleNet,
    },
    /// A line.
    Line {
        /// Range in cells.
        range: AoeRangeNet,
    },
}

impl HitTypeNet {
    /// Mirror the sim's own hit geometry.
    pub(in crate::mcp) fn from_hit_type(hit_type: HitType) -> Self {
        match hit_type {
            HitType::Single => Self::Single,
            HitType::Blast { radius } => Self::Blast {
                radius: BlastRadiusNet::new(*radius),
            },
            HitType::Cone { range, angle } => Self::Cone {
                range: AoeRangeNet::new(*range),
                angle: ConeHalfAngleNet::new(*angle),
            },
            HitType::Line { range } => Self::Line {
                range: AoeRangeNet::new(*range),
            },
        }
    }

    /// Read a client's hit geometry back as the sim's own.
    pub(in crate::mcp) const fn to_hit_type(self) -> HitType {
        match self {
            Self::Single => HitType::Single,
            Self::Blast { radius } => HitType::Blast {
                radius: BlastRadius::new(radius.0),
            },
            Self::Cone { range, angle } => HitType::Cone {
                range: AoeRange::new(range.0),
                angle: ConeHalfAngle::new(angle.0),
            },
            Self::Line { range } => HitType::Line {
                range: AoeRange::new(range.0),
            },
        }
    }
}

/// One fire mode an attachment grants, carrying every field the form's row edits.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(in crate::mcp) struct FireModeSpecNet {
    /// Single, burst or full auto.
    kind:       ModeKindNet,
    /// Cone multiplier.
    cone_mult:  ModeConeMultNet,
    /// TU cost as a fraction of max.
    tu_percent: ModeTuPercentNet,
    /// Shots fired.
    shots:      ModeShotsNet,
    /// Hit geometry.
    hit_type:   HitTypeNet,
}

impl FireModeSpecNet {
    /// Mirror the sim's own fire-mode spec, every field the form's row edits.
    pub(in crate::mcp) fn from_spec(spec: FireModeSpec) -> Self {
        Self {
            kind:       ModeKindNet::from_kind(spec.kind),
            cone_mult:  ModeConeMultNet::new(*spec.cone_mult),
            tu_percent: ModeTuPercentNet::new(*spec.tu_percent),
            shots:      ModeShotsNet::new(*spec.shots),
            hit_type:   HitTypeNet::from_hit_type(spec.hit_type),
        }
    }

    /// Read a client's fire-mode spec back as the sim's own.
    pub(in crate::mcp) const fn to_spec(self) -> FireModeSpec {
        FireModeSpec::with_hit_type(
            self.kind.to_kind(),
            ModeConeMult::new(self.cone_mult.0),
            ModeTuPercent::new(self.tu_percent.0),
            ModeShots::new(self.shots.0),
            self.hit_type.to_hit_type(),
        )
    }
}
