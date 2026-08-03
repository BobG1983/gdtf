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

/// `#[derive(Component)]` — the [`FireMode`] selector that holds the specs is the
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

/// count. Private inner + derived [`Deref`]; `#[serde(transparent)]` so a weapon's
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlastRadius(u8);

impl BlastRadius {
        #[must_use]
    pub const fn new(radius: u8) -> Self {
        Self(radius)
    }
}

/// `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AoeRange(u8);

impl AoeRange {
        #[must_use]
    pub const fn new(range: u8) -> Self {
        Self(range)
    }
}

/// [`Deref`]; `#[serde(transparent)]` so a weapon's RON writes the bare float
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConeHalfAngle(f32);

impl ConeHalfAngle {
        #[must_use]
    pub const fn new(degrees: f32) -> Self {
        Self(degrees)
    }
}

/// a FIELD value on a [`FireModeSpec`] (NOT a `#[derive(Component)]` — the [`FireMode`]
/// [`Single`](Self::Single) is the DEFAULT (`#[serde(default)]` on the
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HitType {
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

impl Default for HitType {
            fn default() -> Self {
        Self::Single
    }
}

/// `#[serde(default)]` = [`HitType::Single`], so every existing weapon `.ron` (which
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FireModeSpec {
            pub kind:       ModeKind,
        pub cone_mult:  ModeConeMult,
        pub tu_percent: ModeTuPercent,
        pub shots:      ModeShots,
        /// `#[serde(default)]` = [`HitType::Single`], so an omitted `hit_type:` field keeps
        #[serde(default)]
    pub hit_type:   HitType,
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

/// `.get()`); `#[serde(transparent)]` so it deserializes from a **bare RON list** of
/// holds a `Vec`). A `#[derive(Component)]` (GTW-200) — the selector lives as a
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
