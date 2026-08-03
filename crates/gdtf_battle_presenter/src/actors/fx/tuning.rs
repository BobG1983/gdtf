use bevy::prelude::*;
use gdtf_assets::HotRonAppExt;
use serde::Deserialize;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ProjectileDrawScale(f32);

impl ProjectileDrawScale {
        pub const DEFAULT: f32 = 0.5;

        #[must_use]
    pub const fn new(fraction: f32) -> Self {
        Self(fraction)
    }
}

impl Default for ProjectileDrawScale {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ProjectileVelocity(f32);

impl ProjectileVelocity {
        pub const DEFAULT: f32 = 480.0;

        #[must_use]
    pub const fn new(pixels_per_second: f32) -> Self {
        Self(pixels_per_second)
    }
}

impl Default for ProjectileVelocity {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct InterShotSeconds(f32);

impl InterShotSeconds {
        pub const DEFAULT: f32 = 0.35;

        #[must_use]
    pub const fn new(seconds: f32) -> Self {
        Self(seconds)
    }
}

impl Default for InterShotSeconds {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ImpactFrameSeconds(f32);

impl ImpactFrameSeconds {
        pub const DEFAULT: f32 = 0.08;

        #[must_use]
    pub const fn new(seconds: f32) -> Self {
        Self(seconds)
    }
}

impl Default for ImpactFrameSeconds {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FctTtlSeconds(f32);

impl FctTtlSeconds {
            pub const DEFAULT: f32 = 1.5;

        #[must_use]
    pub const fn new(seconds: f32) -> Self {
        Self(seconds)
    }
}

impl Default for FctTtlSeconds {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FctRiseRate(f32);

impl FctRiseRate {
            pub const DEFAULT: f32 = 40.0;

        #[must_use]
    pub const fn new(px_per_second: f32) -> Self {
        Self(px_per_second)
    }
}

impl Default for FctRiseRate {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// Each field is `#[serde(default)]` so a `.ron` that omits a field falls back to the
#[derive(Resource, Debug, Clone, Copy, PartialEq, Deserialize, TypePath, Default)]
#[serde(default)]
pub struct FxTuning {
        pub projectile_draw_scale: ProjectileDrawScale,
        pub projectile_velocity:   ProjectileVelocity,
        pub inter_shot_seconds:    InterShotSeconds,
        pub impact_frame_seconds:  ImpactFrameSeconds,
        pub fct_ttl_seconds:       FctTtlSeconds,
        pub fct_rise_rate:         FctRiseRate,
}

const FX_TUNING_RON_PATH: &str = "core_tuning/fx.tuning.ron";

pub(crate) fn register_fx_tuning_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<FxTuning>(FX_TUNING_RON_PATH);
}

#[cfg(test)]
mod test {
    use super::{
        FctRiseRate, FctTtlSeconds, FxTuning, ImpactFrameSeconds, InterShotSeconds,
        ProjectileDrawScale, ProjectileVelocity,
    };

                #[test]
    fn shipped_fx_tuning_ron_parses() {
        const SHIPPED: &str = include_str!("../../../../../assets/core_tuning/fx.tuning.ron");
        let parsed: Result<FxTuning, _> = ron::de::from_str(SHIPPED);
        assert!(
            parsed.is_ok(),
            "shipped fx.tuning.ron must parse into FxTuning, got: {:?}",
            parsed.as_ref().err(),
        );
    }

                    #[test]
    fn default_fx_tuning_matches_the_travel_slice_consts() {
        let tuning = FxTuning::default();
        assert!(
            (*tuning.projectile_draw_scale - ProjectileDrawScale::DEFAULT).abs() < f32::EPSILON,
            "the default projectile draw scale must be the user's 0.5",
        );
        assert!(
            (*tuning.projectile_velocity - ProjectileVelocity::DEFAULT).abs() < f32::EPSILON,
            "the default projectile velocity must be the user's 480 px/sec tune",
        );
        assert!(
            (*tuning.inter_shot_seconds - InterShotSeconds::DEFAULT).abs() < f32::EPSILON,
            "the default burst stagger must be the user's 0.35 s tune",
        );
        assert!(
            (*tuning.impact_frame_seconds - ImpactFrameSeconds::DEFAULT).abs() < f32::EPSILON,
            "the default impact-frame hold must be the TRAVEL-slice 0.08 s",
        );
        assert!(
            (*tuning.fct_ttl_seconds - FctTtlSeconds::DEFAULT).abs() < f32::EPSILON,
            "the default FCT lifetime must be the GTW-327 re-tuned readable window",
        );
        assert!(
            (*tuning.fct_rise_rate - FctRiseRate::DEFAULT).abs() < f32::EPSILON,
            "the default FCT rise rate must be the shipped ascent speed",
        );
    }

            /// `#[serde(default)]` contract that lets a user tune one number without restating the rest.
    #[test]
    fn partial_fx_tuning_ron_falls_back_to_defaults() {
        let parsed: Result<FxTuning, _> = ron::de::from_str("(projectile_velocity: 1234.0)");
        assert!(
            parsed.is_ok(),
            "a partial fx.tuning.ron must parse, got: {:?}",
            parsed.as_ref().err(),
        );
        let Ok(tuning) = parsed else {
            return;
        };
        assert!(
            (*tuning.projectile_velocity - 1234.0).abs() < f32::EPSILON,
            "the authored velocity must win",
        );
        assert!(
            (*tuning.projectile_draw_scale - ProjectileDrawScale::DEFAULT).abs() < f32::EPSILON,
            "an omitted draw scale must fall back to the default",
        );
        assert!(
            (*tuning.inter_shot_seconds - InterShotSeconds::DEFAULT).abs() < f32::EPSILON,
            "an omitted stagger must fall back to the default",
        );
        assert!(
            (*tuning.impact_frame_seconds - ImpactFrameSeconds::DEFAULT).abs() < f32::EPSILON,
            "an omitted impact hold must fall back to the default",
        );
        assert!(
            (*tuning.fct_ttl_seconds - FctTtlSeconds::DEFAULT).abs() < f32::EPSILON,
            "an omitted FCT lifetime must fall back to the default",
        );
        assert!(
            (*tuning.fct_rise_rate - FctRiseRate::DEFAULT).abs() < f32::EPSILON,
            "an omitted FCT rise rate must fall back to the default",
        );
    }
}
