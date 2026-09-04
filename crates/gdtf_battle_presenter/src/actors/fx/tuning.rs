//! Hot-reloadable FX timing and projectile scale.

use bevy::prelude::*;
use cobalt_ron_assets::HotRonAppExt;
use serde::Deserialize;

/// Projectile sprite scale as a fraction of a cell.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ProjectileDrawScale(f32);

impl ProjectileDrawScale {
    /// Shipped default scale.
    pub const DEFAULT: f32 = 0.5;

    /// Build from a scale fraction.
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

/// Projectile flight speed in pixels per second.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ProjectileVelocity(f32);

impl ProjectileVelocity {
    /// Shipped default velocity.
    pub const DEFAULT: f32 = 480.0;

    /// Build from pixels per second.
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

/// Stagger between burst rounds (legacy; cursor dwell owns cadence now).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct InterShotSeconds(f32);

impl InterShotSeconds {
    /// Shipped default stagger.
    pub const DEFAULT: f32 = 0.35;

    /// Build from seconds.
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

/// Hold per impact animation frame.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ImpactFrameSeconds(f32);

impl ImpactFrameSeconds {
    /// Shipped default frame hold.
    pub const DEFAULT: f32 = 0.08;

    /// Build from seconds.
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

/// Floating combat text lifetime.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FctTtlSeconds(f32);

impl FctTtlSeconds {
    /// Shipped default lifetime.
    pub const DEFAULT: f32 = 1.5;

    /// Build from seconds.
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

/// Floating combat text rise speed in pixels per second.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FctRiseRate(f32);

impl FctRiseRate {
    /// Shipped default rise rate.
    pub const DEFAULT: f32 = 40.0;

    /// Build from pixels per second.
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

/// All FX timing knobs, hot-loaded from RON.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Deserialize, TypePath, Default)]
#[serde(default)]
pub struct FxTuning {
    /// Projectile sprite scale.
    pub projectile_draw_scale: ProjectileDrawScale,
    /// Projectile flight speed.
    pub projectile_velocity:   ProjectileVelocity,
    /// Inter-shot stagger (legacy).
    pub inter_shot_seconds:    InterShotSeconds,
    /// Impact frame hold.
    pub impact_frame_seconds:  ImpactFrameSeconds,
    /// Floating text lifetime.
    pub fct_ttl_seconds:       FctTtlSeconds,
    /// Floating text rise rate.
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
            "the default FCT lifetime must be the readable window",
        );
        assert!(
            (*tuning.fct_rise_rate - FctRiseRate::DEFAULT).abs() < f32::EPSILON,
            "the default FCT rise rate must be the shipped ascent speed",
        );
    }

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
