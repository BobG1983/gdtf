use bevy::prelude::Deref;
use serde::Deserialize;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct PenDamageScale(f32);

impl PenDamageScale {
                #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ToughnessMitigation(f32);

impl ToughnessMitigation {
                #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ShooterLuckScale(f32);

impl ShooterLuckScale {
                #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct DefenderLuckScale(f32);

impl DefenderLuckScale {
                #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct RandomSpread(f32);

impl RandomSpread {
                #[must_use]
    pub const fn new(spread: f32) -> Self {
        Self(spread)
    }
}

/// `#[serde(transparent)]` lets it parse a bare RON scalar. Magnitudes are
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct SeverityEdge(f32);

impl SeverityEdge {
                #[must_use]
    pub const fn new(edge: f32) -> Self {
        Self(edge)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SeverityEdges {
        pub e0: SeverityEdge,
        pub e1: SeverityEdge,
        pub e2: SeverityEdge,
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

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SeverityScaling {
        pub pen_damage_scale:     PenDamageScale,
        pub toughness_mitigation: ToughnessMitigation,
        pub shooter_luck_scale:   ShooterLuckScale,
            pub defender_luck_scale:  DefenderLuckScale,
        pub random_spread:        RandomSpread,
        pub edges:                SeverityEdges,
}

impl Default for SeverityScaling {
    fn default() -> Self {
        Self {
            pen_damage_scale:     PenDamageScale::new(1.0),
            toughness_mitigation: ToughnessMitigation::new(1.0),
            shooter_luck_scale:   ShooterLuckScale::new(1.0),
            defender_luck_scale:  DefenderLuckScale::new(1.0),
            random_spread:        RandomSpread::new(10.0),
            edges:                SeverityEdges::default(),
        }
    }
}
