//! The HOT-RELOADABLE edge-pan tuning table and its RON load / resolve / re-derive chain.
//!
//! GTW-250 baked the camera pan speed + the mouse-edge band thickness as Rust `const`s
//! (`PAN_SPEED` / `EDGE_BAND_PX`). GTW-299 migrates them — plus a NEW dwell-delay knob — into
//! a loose, per-line commented `assets/core_tuning/pan.tuning.ron`, loaded through the SAME generic
//! [`RonAsset<T>`](gdtf_assets::RonAsset) loader the theme / situation / tile-roles /
//! effect-roles / FX-tuning tables use, so the user tunes pan speed / edge band / dwell delay
//! WITHOUT a rebuild. This module mirrors the FX-tuning template (`crate::fx::tuning`) EXACTLY:
//! the same spec-IS-the-resolved-resource shape, the same load / resolve / redrive trio.
//!
//! Each value is a NAMED newtype (`.claude/rules/no-bare-types.md`) with a PRIVATE inner read
//! through [`Deref`] and a [`Default`] carrying the GTW-250 shipped value (or, for the dwell, the
//! new `0.3` s choice), so a missing `.ron` field degrades to the prior behaviour rather than a
//! parse error. The three resolve into one presenter-owned [`PanTuning`] resource the pan systems
//! READ (never a `const`), so an edit to the `.ron` re-derives [`PanTuning`] live
//! through the GTW-564 generic hot-RON registration helper ([`register_pan_tuning_hot_ron`]) —
//! mirroring the FX table's [`FxTuning`](crate::FxTuning) hot-reload.

use bevy::prelude::*;
use gdtf_assets::HotRonAppExt;
use serde::Deserialize;

use super::pan::{EdgeBandPx, PanSpeed};

/// How long the cursor must LINGER inside the edge band before edge-pan starts, in SECONDS.
///
/// GTW-299: edge-pan no longer fires the instant the cursor crosses into the edge band — it must
/// dwell there for at least this long first, so a cursor that merely brushes the edge (e.g. on a
/// click-release near the map border) does NOT yank the camera; only a deliberate rest at the edge
/// pans. The default `0.3` s is the user's explicit dwell choice — long enough to ignore an
/// incidental graze, short enough that an intentional edge-rest pans promptly — and is live-tunable
/// through `assets/core_tuning/pan.tuning.ron`.
///
/// A named newtype over the `f32` seconds (`.claude/rules/no-bare-types.md`): the inner is PRIVATE,
/// read through [`Deref`] and built through [`new`](Self::new) / [`Default`].
/// `#[serde(transparent)]` so the `.ron` authors the inner number directly.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct DwellDelaySeconds(f32);

impl DwellDelaySeconds {
    /// The shipped default: `0.3` s of edge-band dwell before edge-pan starts — the user's choice.
    pub const DEFAULT: f32 = 0.3;

    /// Build a dwell delay from a seconds duration.
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

/// How far PAST the battlefield edge the camera may pan, as a FIXED world-space distance.
///
/// GTW-381: the bounds clamp ([`clamp_camera_to_bounds`](super::clamp_camera_to_bounds)) no
/// longer pins the camera AT the hard battlefield edge — it relaxes the clamp by this much,
/// so the camera CAN pan up to this many world units past the level edge (a little breathing
/// room around the map) and is clamped at `bounds + margin`, never further and never at the
/// old hard bounds (bounded + padding, not unbounded).
///
/// A FIXED WORLD-SPACE distance, deliberately INDEPENDENT of zoom: it is fed into the existing
/// zoom-aware clamp by GROWING the world bounds box BEFORE the clamp's per-axis math runs, so
/// the same world-space slack is allowed at ANY zoom level — it does not scale with the
/// viewport (the half-viewport already carries the zoom; this margin does not, by design, C3).
/// One cell is [`CELL_PX`](crate::CELL_PX) (`16`) world units wide, so the shipped `128`
/// default is roughly EIGHT cells of off-level slack — a comfortable, visible amount of pan
/// headroom past the edge without letting the map drift arbitrarily far off-screen.
///
/// A named newtype over the `f32` world distance (`.claude/rules/no-bare-types.md`): the inner
/// is PRIVATE, read through [`Deref`] and built through [`new`](Self::new) / [`Default`].
/// `#[serde(transparent)]` so the `.ron` authors the inner number directly; the live consumer
/// is [`clamp_camera_to_bounds`](super::clamp_camera_to_bounds), which reads it off
/// [`PanTuning::bounds_margin_world`] and adds it to the world bounds — no dead leaf.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BoundsMarginWorld(f32);

impl BoundsMarginWorld {
    /// The shipped default: `128` world units (~8 cells at the `16`-unit cell pitch) of slack
    /// past the battlefield edge — a sensible, visible amount of off-level pan headroom that is
    /// still bounded (the relaxed clamp pins the camera at `bounds + margin`, never further).
    pub const DEFAULT: f32 = 128.0;

    /// Build an off-level pan margin from a world-space distance (world units).
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

/// The HOT-RELOADABLE edge-pan tuning table — the GTW-250 pan speed + edge band plus the GTW-299
/// dwell delay, loaded from `assets/core_tuning/pan.tuning.ron` and read live by the pan systems.
///
/// Loaded from the loose `assets/core_tuning/pan.tuning.ron` through the GTW-564 generic
/// hot-RON chain (`register_pan_tuning_hot_ron`) and resolved into a presenter-owned
/// [`PanTuning`] resource, then re-derived in place on a hot edit — the SAME dual-role
/// spec-IS-the-resolved-resource shape [`FxTuning`](crate::FxTuning) uses (the value
/// clones straight out of the `RonAsset`, no extra resolve step).
///
/// Each field is `#[serde(default)]` so a `.ron` that omits a field falls back to the shipped
/// value rather than failing to parse — an author can tune one number without restating the rest.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored `.ron` shape),
/// [`Default`] (the all-shipped-values fallback the headless app and a partial `.ron` use), and
/// [`TypePath`] (the bound [`RonAsset<PanTuning>`](gdtf_assets::RonAsset) requires of its payload).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Deserialize, TypePath, Default)]
#[serde(default)]
pub struct PanTuning {
    /// The mouse / gamepad-cursor edge-band thickness (logical pixels).
    pub edge_band_px:        EdgeBandPx,
    /// The camera pan speed (world units per second).
    pub pan_speed:           PanSpeed,
    /// The cursor edge-band dwell before edge-pan starts (seconds).
    pub dwell_delay_seconds: DwellDelaySeconds,
    /// How far the camera may pan PAST the battlefield edge (fixed world-space distance, GTW-381).
    pub bounds_margin_world: BoundsMarginWorld,
}

/// The path of the loose pan-tuning RON, relative to the asset source root.
const PAN_TUNING_RON_PATH: &str = "core_tuning/pan.tuning.ron";

/// Registers the [`PanTuning`] hot-RON chain — ONE ext call onto the GTW-564
/// generic registration helper (kick-off / gated resolve / live redrive, keyed by the generic
/// [`HotRonHandle`](gdtf_assets::HotRonHandle)`<PanTuning>`), replacing the
/// per-site handle newtype + load/resolve/redrive triple. Self-gates on the
/// [`AssetServer`](bevy::asset::AssetServer) (`bevy-traps.md` #1), so a
/// `MinimalPlugins` headless app stays a no-op. On a live `.ron` edit the
/// generic redrive overwrites [`PanTuning`] through `ResMut`, so the pan
/// systems read the new speed / edge band / dwell delay the very next frame —
/// WITHOUT a rebuild.
pub(crate) fn register_pan_tuning_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<PanTuning>(PAN_TUNING_RON_PATH);
}

#[cfg(test)]
mod test {
    use super::{BoundsMarginWorld, DwellDelaySeconds, PanTuning};
    use crate::{EdgeBandPx, PanSpeed};

    /// The shipped `pan.tuning.ron` parses into `PanTuning` and carries every tuning value —
    /// a `ron::de` round-trip of the SHIPPED bytes (a missing-but-required field would be a
    /// deserialize error; an absent field falls back to its `Default`).
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

    /// `PanTuning::default()` reproduces the user's existing pan-speed / edge-band values EXACTLY
    /// plus the new dwell default — the migration must not silently change a tuning value (the `400`
    /// speed, the `24` edge band, the `0.3` dwell; the `DEFAULT` consts ARE those shipped values, so
    /// this guards the newtype `Default` impls against drift).
    ///
    /// This also pins the AC4 floor against drift from the user's existing behaviour: the pan-speed
    /// default is asserted to be the user's `400` (not the self-referential `PanSpeed::DEFAULT`), so
    /// the migration cannot quietly retune the camera glide. `400` (not the old GTW-250 `600`) is the
    /// user's deliberate pan-feel tune that was the ACTUAL working-tree behaviour pre-migration, so
    /// `400` IS "existing behaviour unchanged" — AC4's literal `600` predates that tune.
    #[test]
    fn default_pan_tuning_matches_the_consts() {
        /// The user's existing pan-speed value (`PanSpeed::new(400.0)`), pinned as a literal so this
        /// test fails if the migrated default drifts off the user's deliberate pan-feel tune — AC4.
        const PRIOR_PAN_SPEED: f32 = 400.0;
        /// The GTW-250 edge-band const on `develop` (`EdgeBandPx::new(24.0)`), pinned as a literal.
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
        // GTW-381: the off-level pan margin default is the shipped BoundsMarginWorld::DEFAULT,
        // and it is non-zero (the feature is live out of the box — the camera CAN pan past the
        // edge by default). The exact magnitude is a tunable, so only the field == DEFAULT
        // relation and the non-zero property are asserted, not a pinned scene value.
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

    /// A PARTIAL `.ron` (one field authored, the rest omitted) parses, taking the authored value
    /// and falling back to the shipped default for every omitted field — the `#[serde(default)]`
    /// contract that lets a user tune one number without restating the rest. This exercises the
    /// dwell-delay path on the real deserialize: the authored `0.75` wins, while speed + edge band
    /// fall back to their defaults.
    #[test]
    fn partial_pan_tuning_ron_falls_back_to_defaults() {
        // Only the dwell delay is authored; the other two must fall back to their defaults.
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
