//! Severity curve scales and edges.

use bevy::prelude::Deref;
use serde::Deserialize;

/// Scale on penetration vs damage.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct PenDamageScale(f32);

impl PenDamageScale {
    /// Wrap a scale.
    #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

/// Toughness mitigation scale.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ToughnessMitigation(f32);

impl ToughnessMitigation {
    /// Wrap a scale.
    #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

/// Shooter luck contribution scale.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ShooterLuckScale(f32);

impl ShooterLuckScale {
    /// Wrap a scale.
    #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

/// Defender luck contribution scale.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct DefenderLuckScale(f32);

impl DefenderLuckScale {
    /// Wrap a scale.
    #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

/// Random spread width on the severity roll.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct RandomSpread(f32);

impl RandomSpread {
    /// Wrap a spread value.
    #[must_use]
    pub const fn new(spread: f32) -> Self {
        Self(spread)
    }
}

/// One threshold on the severity curve.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct SeverityEdge(f32);

impl SeverityEdge {
    /// Wrap an edge value.
    #[must_use]
    pub const fn new(edge: f32) -> Self {
        Self(edge)
    }
}

/// Four severity band edges.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SeverityEdges {
    /// First edge.
    pub e0: SeverityEdge,
    /// Second edge.
    pub e1: SeverityEdge,
    /// Third edge.
    pub e2: SeverityEdge,
    /// Fourth edge.
    pub e3: SeverityEdge,
}

impl Default for SeverityEdges {
    fn default() -> Self {
        Self {
            e0: SeverityEdge::new(1.0),
            e1: SeverityEdge::new(5.0),
            e2: SeverityEdge::new(10.0),
            e3: SeverityEdge::new(15.0),
        }
    }
}

/// Full severity scaling bundle.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SeverityScaling {
    /// Penetration / damage scale.
    pub pen_damage_scale: PenDamageScale,
    /// Toughness mitigation.
    pub toughness_mitigation: ToughnessMitigation,
    /// Shooter luck scale.
    pub shooter_luck_scale: ShooterLuckScale,
    /// Defender luck scale.
    pub defender_luck_scale: DefenderLuckScale,
    /// Random spread.
    pub random_spread: RandomSpread,
    /// Band edges.
    pub edges: SeverityEdges,
}

impl Default for SeverityScaling {
    fn default() -> Self {
        Self {
            pen_damage_scale: PenDamageScale::new(1.0),
            toughness_mitigation: ToughnessMitigation::new(1.0),
            shooter_luck_scale: ShooterLuckScale::new(1.0),
            defender_luck_scale: DefenderLuckScale::new(1.0),
            random_spread: RandomSpread::new(10.0),
            edges: SeverityEdges::default(),
        }
    }
}
