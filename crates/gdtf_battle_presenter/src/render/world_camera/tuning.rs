use bevy::prelude::*;
use gdtf_assets::HotRonAppExt;
use serde::Deserialize;

use super::pan::{EdgeBandPx, PanSpeed};

/// `#[serde(transparent)]` so the `.ron` authors the inner number directly.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct DwellDelaySeconds(f32);

impl DwellDelaySeconds {
        pub const DEFAULT: f32 = 0.3;

        #[must_use]
    pub const fn new(seconds: f32) -> Self {
        Self(seconds)
    }
}

impl Default for DwellDelaySeconds {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// `#[serde(transparent)]` so the `.ron` authors the inner number directly; the live consumer
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BoundsMarginWorld(f32);

impl BoundsMarginWorld {
                pub const DEFAULT: f32 = 128.0;

        #[must_use]
    pub const fn new(world_units: f32) -> Self {
        Self(world_units)
    }
}

impl Default for BoundsMarginWorld {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// Each field is `#[serde(default)]` so a `.ron` that omits a field falls back to the shipped
#[derive(Resource, Debug, Clone, Copy, PartialEq, Deserialize, TypePath, Default)]
#[serde(default)]
pub struct PanTuning {
        pub edge_band_px:        EdgeBandPx,
        pub pan_speed:           PanSpeed,
        pub dwell_delay_seconds: DwellDelaySeconds,
        pub bounds_margin_world: BoundsMarginWorld,
}

const PAN_TUNING_RON_PATH: &str = "core_tuning/pan.tuning.ron";

pub(crate) fn register_pan_tuning_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<PanTuning>(PAN_TUNING_RON_PATH);
}

#[cfg(test)]
mod test {
    use super::{BoundsMarginWorld, DwellDelaySeconds, PanTuning};
    use crate::{EdgeBandPx, PanSpeed};

                #[test]
    fn shipped_pan_tuning_ron_parses() {
        const SHIPPED: &str = include_str!("../../../../../assets/core_tuning/pan.tuning.ron");
        let parsed: Result<PanTuning, _> = ron::de::from_str(SHIPPED);
        assert!(
            parsed.is_ok(),
            "shipped pan.tuning.ron must parse into PanTuning, got: {:?}",
            parsed.as_ref().err(),
        );
    }

                                            #[test]
    fn default_pan_tuning_matches_the_consts() {
                        const PRIOR_PAN_SPEED: f32 = 400.0;
                const PRIOR_EDGE_BAND_PX: f32 = 24.0;

        let tuning = PanTuning::default();
        assert!(
            (*tuning.edge_band_px - PRIOR_EDGE_BAND_PX).abs() < f32::EPSILON,
            "the default edge band must be the prior 24 px const",
        );
        assert!(
            (EdgeBandPx::DEFAULT - PRIOR_EDGE_BAND_PX).abs() < f32::EPSILON,
            "EdgeBandPx::DEFAULT must equal the prior 24 px const",
        );
        assert!(
            (*tuning.pan_speed - PRIOR_PAN_SPEED).abs() < f32::EPSILON,
            "the default pan speed must be the user's 400 units/sec tune (AC4: unchanged)",
        );
        assert!(
            (PanSpeed::DEFAULT - PRIOR_PAN_SPEED).abs() < f32::EPSILON,
            "PanSpeed::DEFAULT must equal the user's 400 units/sec tune (AC4: unchanged)",
        );
        assert!(
            (*tuning.dwell_delay_seconds - DwellDelaySeconds::DEFAULT).abs() < f32::EPSILON,
            "the default dwell delay must be the user's 0.3 s",
        );
        assert!(
            (*tuning.bounds_margin_world - BoundsMarginWorld::DEFAULT).abs() < f32::EPSILON,
            "the default off-level margin must be BoundsMarginWorld::DEFAULT",
        );
        const {
            assert!(
                BoundsMarginWorld::DEFAULT > 0.0,
                "the shipped off-level margin must be non-zero so the relaxed clamp is live by default",
            );
        }
    }

        /// and falling back to the shipped default for every omitted field — the `#[serde(default)]`
                #[test]
    fn partial_pan_tuning_ron_falls_back_to_defaults() {
        let parsed: Result<PanTuning, _> = ron::de::from_str("(dwell_delay_seconds: 0.75)");
        assert!(
            parsed.is_ok(),
            "a partial pan.tuning.ron must parse, got: {:?}",
            parsed.as_ref().err(),
        );
        let Ok(tuning) = parsed else {
            return;
        };
        assert!(
            (*tuning.dwell_delay_seconds - 0.75).abs() < f32::EPSILON,
            "the authored dwell delay must win",
        );
        assert!(
            (*tuning.edge_band_px - EdgeBandPx::DEFAULT).abs() < f32::EPSILON,
            "an omitted edge band must fall back to the default",
        );
        assert!(
            (*tuning.pan_speed - PanSpeed::DEFAULT).abs() < f32::EPSILON,
            "an omitted pan speed must fall back to the default",
        );
        assert!(
            (*tuning.bounds_margin_world - BoundsMarginWorld::DEFAULT).abs() < f32::EPSILON,
            "an omitted off-level margin must fall back to the default",
        );
    }
}
