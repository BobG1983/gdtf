use bevy::prelude::Deref;

use crate::{
    effects::attachments::WeaponBraceBonus, stability::terrain_brace::TerrainBraced, weapon::Stable,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StabilityTerms {
        pub stable:         Stable,
        pub terrain_braced: TerrainBraced,
            pub brace_bonus:    WeaponBraceBonus,
            pub emplacement:    EmplacementStability,
}

impl Default for StabilityTerms {
                fn default() -> Self {
        Self {
            stable:         Stable::new(false),
            terrain_braced: TerrainBraced::new(false),
            brace_bonus:    WeaponBraceBonus::none(),
            emplacement:    EmplacementStability::none(),
        }
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct EmplacementStability(f32);

impl EmplacementStability {
        #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }

            #[must_use]
    pub const fn none() -> Self {
        Self(0.0)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct SuppressionStability(f32);

impl SuppressionStability {
                        #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }

                #[must_use]
    pub const fn none() -> Self {
        Self(0.0)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct StabilityScore(f32);

impl StabilityScore {
        const MIN: f32 = 0.0;
                pub(super) const MAX: f32 = 100.0;

                    #[must_use]
    pub const fn clamped(raw: f32) -> Self {
        Self(raw.clamp(Self::MIN, Self::MAX))
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ConeMult(f32);

impl ConeMult {
        #[must_use]
    pub const fn new(mult: f32) -> Self {
        Self(mult)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct RecoilGrowth(f32);

impl RecoilGrowth {
            #[must_use]
    pub const fn new(coeff: f32) -> Self {
        Self(coeff)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub(super) struct CurveOutput(f32);

impl CurveOutput {
            pub(super) const IDENTITY: Self = Self(1.0);

                pub(super) const fn new(output: f32) -> Self {
        Self(output)
    }
}
