//! The HOT-RELOADABLE edge-pan tuning table and its RON load / resolve / re-derive chain.
//!
//! GTW-250 baked the camera pan speed + the mouse-edge band thickness as Rust `const`s
//! (`PAN_SPEED` / `EDGE_BAND_PX`). GTW-299 migrates them — plus a NEW dwell-delay knob — into
//! a loose, per-line commented `assets/tiles/pan_tuning.ron`, loaded through the SAME generic
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
//! ([`redrive_pan_tuning_on_asset_event`]) — mirroring the FX table's
//! [`redrive_fx_tuning_on_asset_event`](crate::redrive_fx_tuning_on_asset_event) hot-reload.

use bevy::{asset::AssetEvent, prelude::*};
use gdtf_assets::RonAsset;
use serde::Deserialize;

use super::pan::{EdgeBandPx, PanSpeed};

/// How long the cursor must LINGER inside the edge band before edge-pan starts, in SECONDS.
///
/// GTW-299: edge-pan no longer fires the instant the cursor crosses into the edge band — it must
/// dwell there for at least this long first, so a cursor that merely brushes the edge (e.g. on a
/// click-release near the map border) does NOT yank the camera; only a deliberate rest at the edge
/// pans. The default `0.3` s is the user's explicit dwell choice — long enough to ignore an
/// incidental graze, short enough that an intentional edge-rest pans promptly — and is live-tunable
/// through `assets/tiles/pan_tuning.ron`.
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

/// The HOT-RELOADABLE edge-pan tuning table — the GTW-250 pan speed + edge band plus the GTW-299
/// dwell delay, loaded from `assets/tiles/pan_tuning.ron` and read live by the pan systems.
///
/// Loaded from the loose `assets/tiles/pan_tuning.ron` through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-owned [`PanTuning`]
/// resource ([`resolve_pan_tuning`]), then re-derived in place on a hot edit
/// ([`redrive_pan_tuning_on_asset_event`]) — the SAME dual-role spec-IS-the-resolved-resource
/// shape [`FxTuning`](crate::FxTuning) uses (the value clones straight out of the `RonAsset`, no
/// extra resolve step).
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
}

/// The path of the loose pan-tuning RON, relative to the asset source root.
const PAN_TUNING_RON_PATH: &str = "tiles/pan_tuning.ron";

/// The in-flight handle to the pan-tuning RON, held until it resolves into [`PanTuning`].
///
/// A named newtype over the bevy [`Handle`] (`.claude/rules/no-bare-types.md`: a bare handle
/// carries no domain meaning; this name says "the pan-tuning table being loaded"). Inserted by
/// [`load_pan_tuning`] and read by [`resolve_pan_tuning`] / [`redrive_pan_tuning_on_asset_event`],
/// mirroring the [`FxTuningHandle`](crate::FxTuningHandle).
#[derive(Resource, Deref, Debug, Clone)]
pub struct PanTuningHandle(Handle<RonAsset<PanTuning>>);

impl PanTuningHandle {
    /// Wrap the in-flight pan-tuning RON handle.
    #[must_use]
    pub const fn new(handle: Handle<RonAsset<PanTuning>>) -> Self {
        Self(handle)
    }
}

/// `Startup`: kick off the `pan_tuning.ron` load, storing its typed handle.
///
/// Loads `tiles/pan_tuning.ron` as a [`RonAsset<PanTuning>`](gdtf_assets::RonAsset) through the
/// generic loader and inserts the [`PanTuningHandle`] the [`resolve_pan_tuning`] poll +
/// [`redrive_pan_tuning_on_asset_event`] hot-reload systems read. Takes `Option<Res<AssetServer>>`
/// so a `MinimalPlugins` headless app with no [`AssetServer`] no-ops rather than panicking
/// (`bevy-traps.md` #1); under `DefaultPlugins` the load fires for real.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the handle insert, the optional
/// [`Res<AssetServer>`] for the load.
pub fn load_pan_tuning(mut commands: Commands, asset_server: Option<Res<AssetServer>>) {
    let Some(asset_server) = asset_server else {
        return;
    };
    let handle = asset_server.load::<RonAsset<PanTuning>>(PAN_TUNING_RON_PATH);
    commands.insert_resource(PanTuningHandle::new(handle));
}

/// `Update` (gated until [`PanTuning`] is resolved): resolve the loaded RON into the
/// presenter-owned [`PanTuning`] resource.
///
/// Once the [`RonAsset<PanTuning>`](gdtf_assets::RonAsset) has settled into
/// `Assets<RonAsset<PanTuning>>`, it clones the deserialized [`PanTuning`] out and inserts it as
/// the resident resource so it is present before the first pan. Run only while [`PanTuningHandle`]
/// exists AND [`PanTuning`] does NOT (the plugin's run-condition), so it inserts once; the live
/// re-derive is [`redrive_pan_tuning_on_asset_event`]. Mirrors
/// [`resolve_fx_tuning`](crate::resolve_fx_tuning).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert, [`Res<PanTuningHandle>`] for the
/// handle, [`Res<Assets<RonAsset<PanTuning>>>`] for the loaded asset.
pub fn resolve_pan_tuning(
    mut commands: Commands,
    handle: Res<PanTuningHandle>,
    tuning_assets: Res<Assets<RonAsset<PanTuning>>>,
) {
    let Some(loaded) = tuning_assets.get(&**handle) else {
        // Loaded-but-not-yet-in-collection (or still loading) — retry next frame; the
        // run-condition keeps this system alive until PanTuning is resolved.
        return;
    };
    commands.insert_resource(**loaded);
}

/// `Update`: re-derive [`PanTuning`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) — the LIVE hot-reload.
///
/// Reads the [`MessageReader`] of
/// [`AssetEvent`](bevy::asset::AssetEvent)`<`[`RonAsset`]`<`[`PanTuning`]`>>` — asset events are
/// MESSAGES in Bevy 0.18, so this is a `MessageReader`, not an `EventReader` (`bevy-traps.md` #4) —
/// and acts only on a [`Modified`](bevy::asset::AssetEvent::Modified) event whose `id` matches the
/// in-flight [`PanTuningHandle`]; events for any other handle are ignored. On a match it clones the
/// refreshed [`PanTuning`] out of the `Assets` collection and overwrites the resident resource
/// through [`ResMut`], so the pan systems read the new speed / edge band / dwell delay the very next
/// frame — WITHOUT a rebuild. Mirrors [`redrive_fx_tuning_on_asset_event`](crate::redrive_fx_tuning_on_asset_event).
///
/// Guarded so it never panics before the load chain has run (pre-resolve): it takes the handle /
/// the `Assets` collection / the [`PanTuning`] resource as [`Option`]al borrows, draining the
/// reader and returning early if any is missing (`bevy-traps.md` #1) so a pre-resolve event does
/// not linger and re-fire later.
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional handle / `Assets` /
/// [`PanTuning`] borrows.
pub fn redrive_pan_tuning_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<PanTuning>>>,
    handle: Option<Res<PanTuningHandle>>,
    tuning_assets: Option<Res<Assets<RonAsset<PanTuning>>>>,
    tuning: Option<ResMut<PanTuning>>,
) {
    let (Some(handle), Some(tuning_assets), Some(mut tuning)) = (handle, tuning_assets, tuning)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to re-derive yet.
        events.clear();
        return;
    };

    let active_id = handle.id();
    // Act once per frame even if several Modified events arrive: a single re-derive from
    // the latest in-memory value covers them all.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { id } if *id == active_id));
    if !modified {
        return;
    }

    let Some(updated) = tuning_assets.get(&**handle) else {
        // Modified but not currently in the collection (a transient reload state) — leave
        // the existing tuning until it settles; the next event re-fires.
        return;
    };
    *tuning = **updated;
    // GTW-374 Part C: log EVERY hot-reload path naming what reloaded, so a live edit can
    // be traced (mirrors the theme / combat-log / B1-B3 handlers).
    info!("pan hot-reload: re-derived PanTuning from `tiles/pan_tuning.ron`");
}

#[cfg(test)]
mod test {
    use super::{DwellDelaySeconds, PanTuning};
    use crate::{EdgeBandPx, PanSpeed};

    /// The shipped `pan_tuning.ron` parses into `PanTuning` and carries every tuning value —
    /// a `ron::de` round-trip of the SHIPPED bytes (a missing-but-required field would be a
    /// deserialize error; an absent field falls back to its `Default`).
    #[test]
    fn shipped_pan_tuning_ron_parses() {
        const SHIPPED: &str = include_str!("../../../../assets/tiles/pan_tuning.ron");
        let parsed: Result<PanTuning, _> = ron::de::from_str(SHIPPED);
        assert!(
            parsed.is_ok(),
            "shipped pan_tuning.ron must parse into PanTuning, got: {:?}",
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
            "a partial pan_tuning.ron must parse, got: {:?}",
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
    }
}
