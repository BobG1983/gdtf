//! The HOT-RELOADABLE FX-tuning table and its RON load / resolve / re-derive chain.
//!
//! GTW-306's TRAVEL slice baked four firing-FX tuning numbers as Rust `const`s
//! (the projectile draw scale, its flight velocity, the burst stagger step, and
//! the per-impact-frame hold). This slice migrates them into a loose, per-line
//! commented `assets/core_tuning/fx.tuning.ron`, loaded through the SAME generic
//! [`RonAsset<T>`](gdtf_assets::RonAsset) loader the theme / situation / tile-roles
//! / effect-roles tables use, so the user tunes size / velocity / stagger /
//! impact-timing WITHOUT a rebuild.
//!
//! Each value is a NAMED newtype (`.claude/rules/no-bare-types.md`) with a PRIVATE
//! inner read through [`Deref`] and a [`Default`] carrying the TRAVEL slice's
//! shipped value, so a missing `.ron` field degrades to the prior behaviour rather
//! than a parse error. The four resolve into one presenter-owned [`FxTuning`]
//! resource the projectile + impact systems READ (never a `const`), so an edit to
//! the `.ron` re-derives [`FxTuning`] live through the GTW-564 generic hot-RON
//! registration helper ([`register_fx_tuning_hot_ron`]) — mirroring the UI theme hot-reload.

use bevy::prelude::*;
use gdtf_assets::HotRonAppExt;
use serde::Deserialize;

/// The traveling projectile's draw size, as a UNIFORM fraction of `CELL_PX`.
///
/// The directional comet glyph occupies only ~7px of its 16px tile, so at 1× it
/// reads as a generic round dot rather than a clearly directional bolt; drawing the
/// tile uniformly larger (the SAME factor on both axes — NEVER stretched along the
/// shot vector, which was the GTW-290 smear bug) makes the comet head + tapered tail
/// legible. The default `0.5` is the user's TRAVEL-slice size tune (kept verbatim).
///
/// A named newtype over the `f32` multiplier (`.claude/rules/no-bare-types.md`): the
/// inner is PRIVATE, read through [`Deref`] and built through [`new`](Self::new) /
/// [`Default`]. The multiplier is FRAMEWORK plumbing the system feeds straight to
/// `custom_size`, but it is a TUNABLE the `.ron` authors, so it earns a name.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ProjectileDrawScale(f32);

impl ProjectileDrawScale {
    /// The TRAVEL-slice default: `0.5` of `CELL_PX` (the user's size tune).
    pub const DEFAULT: f32 = 0.5;

    /// Build a draw scale from a uniform `CELL_PX` fraction.
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

/// How fast every projectile flies, in WORLD PIXELS PER SECOND.
///
/// The bolt travels at a CONSTANT velocity rather than over a fixed-seconds window,
/// so every shot shares one visual SPEED regardless of distance (a far shot simply
/// spends more frames in flight). The default `480` ≈ 30 cells/s
/// ([`CELL_PX`](crate::CELL_PX) = 16) — the user's current working-tree tune (the
/// TRAVEL slice shipped `960`; the user has since slowed it).
///
/// A named newtype over the `f32` speed (`.claude/rules/no-bare-types.md`): the inner
/// is PRIVATE, read through [`Deref`] and built through [`new`](Self::new) /
/// [`Default`]. The live per-projectile flight state IS a separate newtype
/// ([`ProjectileTravel`](super::projectile::ProjectileTravel)); this is the tunable
/// speed it advances by.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ProjectileVelocity(f32);

impl ProjectileVelocity {
    /// The shipped default: `480` px/sec (≈ 30 cells/s) — the user's current tune.
    pub const DEFAULT: f32 = 480.0;

    /// Build a velocity from a world-pixels-per-second speed.
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

/// The gap between successive rounds of one burst / full-auto shot leaving the
/// muzzle, in SECONDS — the per-round LAUNCH-DELAY step that staggers a volley.
///
/// A burst emits one [`ShotFired`](gdtf_battle_sim::shot_fired::ShotFired) per round in a single
/// frame, so without a stagger every projectile would launch at once and read as one
/// fat bolt. Each round's read-order index × this step is its launch delay, so the
/// volley animates SHOT-BY-SHOT. The default `0.35` s is the user's current
/// working-tree tune (the TRAVEL slice shipped `0.1`; the user has since widened it),
/// long enough that successive rounds are clearly distinct.
///
/// A named newtype over the `f32` seconds (`.claude/rules/no-bare-types.md`): the
/// inner is PRIVATE, read through [`Deref`] and built through [`new`](Self::new) /
/// [`Default`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct InterShotSeconds(f32);

impl InterShotSeconds {
    /// The shipped default: `0.35` s between burst rounds — the user's current tune.
    pub const DEFAULT: f32 = 0.35;

    /// Build a stagger step from a seconds gap.
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

/// How long each impact ANIMATION frame stays on screen, in SECONDS.
///
/// A small, fixed per-frame hold so the three impact tiles read as a fast expanding
/// shockwave; `IMPACT_FRAME_COUNT` of these is the whole impact. The default `0.08` s
/// is in-step with the firing FX.
///
/// A named newtype over the `f32` seconds (`.claude/rules/no-bare-types.md`): the
/// inner is PRIVATE, read through [`Deref`] and built through [`new`](Self::new) /
/// [`Default`]. The live per-frame clock IS a separate newtype
/// (`ImpactAnimation`); this is the tunable hold it
/// runs each frame for.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ImpactFrameSeconds(f32);

impl ImpactFrameSeconds {
    /// The TRAVEL-slice default: `0.08` s per impact frame.
    pub const DEFAULT: f32 = 0.08;

    /// Build an impact-frame hold from a seconds duration.
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

/// How long one floating-combat-text (FCT) pop lives before it despawns, in SECONDS.
///
/// A pop rises + fades across this whole window and despawns the moment its clock
/// finishes. GTW-327 RE-TUNE: the migrated default is `1.5` s — a readable window
/// (the original `0.6` s faded too fast to read) — and, now that it is a hot-reloadable
/// [`FxTuning`] field, the user dials it live in `fx.tuning.ron` without a rebuild. The
/// live per-pop clock is a separate concern
/// ([`FloatingCombatText`](super::fct::FloatingCombatText)); this is the tunable lifetime
/// it runs for.
///
/// A named newtype over the `f32` seconds (`.claude/rules/no-bare-types.md`): the inner
/// is PRIVATE, read through [`Deref`] and built through [`new`](Self::new) / [`Default`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FctTtlSeconds(f32);

impl FctTtlSeconds {
    /// The GTW-327 re-tuned default: `1.5` s — a window long enough to read (the original
    /// `0.6` s faded too fast); RON-tunable so the user can dial it live.
    pub const DEFAULT: f32 = 1.5;

    /// Build a pop-lifetime in seconds.
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

/// How fast a floating-combat-text (FCT) pop ASCENDS, in world px per second.
///
/// The pop's `y` grows by this × elapsed life each frame — a legible drift upward
/// without flying off the unit. The default `40` px/s lifts the pop a few cells over the
/// (now `1.5` s) lifetime, a slow, readable climb that does not overshoot the cell it is
/// reporting on. Now a hot-reloadable [`FxTuning`] field so the user tunes the climb live.
///
/// A named newtype over the `f32` px/sec (`.claude/rules/no-bare-types.md`): the inner is
/// PRIVATE, read through [`Deref`] and built through [`new`](Self::new) / [`Default`]. The
/// live per-pop ascent state IS the separate
/// [`FloatingCombatText`](super::fct::FloatingCombatText); this is the tunable rate it
/// advances by.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FctRiseRate(f32);

impl FctRiseRate {
    /// The default ascent speed — `40` px/s; a slow, readable climb that, across the
    /// re-tuned `1.5` s lifetime, drifts the pop a few cells up without overshooting.
    pub const DEFAULT: f32 = 40.0;

    /// Build a rise rate in world px per second.
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

/// The HOT-RELOADABLE firing-FX tuning table — the four TRAVEL-slice tuning numbers,
/// loaded from `assets/core_tuning/fx.tuning.ron` and read live by the projectile + impact
/// systems.
///
/// Loaded from the loose `assets/core_tuning/fx.tuning.ron` through the GTW-564
/// generic hot-RON chain (`register_fx_tuning_hot_ron`) and resolved into a
/// presenter-owned [`FxTuning`] resource, then re-derived in place on a hot
/// edit — the SAME dual-role spec-IS-the-resolved-resource shape
/// [`EffectRoles`](super::roles::EffectRoles) uses (the value clones straight
/// out of the `RonAsset`, no extra resolve step).
///
/// Each field is `#[serde(default)]` so a `.ron` that omits a field falls back to the
/// TRAVEL-slice shipped value rather than failing to parse — an author can tune one
/// number without restating the rest.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored
/// `.ron` shape), [`Default`] (the all-shipped-values fallback the headless app and
/// a partial `.ron` use), and [`TypePath`] (the bound
/// [`RonAsset<FxTuning>`](gdtf_assets::RonAsset) requires of its payload).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Deserialize, TypePath, Default)]
#[serde(default)]
pub struct FxTuning {
    /// The traveling projectile's UNIFORM draw scale (fraction of `CELL_PX`).
    pub projectile_draw_scale: ProjectileDrawScale,
    /// The projectile's constant flight velocity (world px/sec).
    pub projectile_velocity:   ProjectileVelocity,
    /// The burst stagger step between successive rounds (seconds).
    pub inter_shot_seconds:    InterShotSeconds,
    /// The per-impact-frame hold (seconds).
    pub impact_frame_seconds:  ImpactFrameSeconds,
    /// The floating-combat-text pop lifetime (seconds) — GTW-327 re-tune to a readable window.
    pub fct_ttl_seconds:       FctTtlSeconds,
    /// The floating-combat-text pop ascent speed (world px/sec).
    pub fct_rise_rate:         FctRiseRate,
}

/// The path of the loose FX-tuning RON, relative to the asset source root.
const FX_TUNING_RON_PATH: &str = "core_tuning/fx.tuning.ron";

/// Registers the [`FxTuning`] hot-RON chain — ONE ext call onto the GTW-564
/// generic registration helper (kick-off / gated resolve / live redrive, keyed by the generic
/// [`HotRonHandle`](gdtf_assets::HotRonHandle)`<FxTuning>`), replacing the
/// per-site handle newtype + load/resolve/redrive triple. Self-gates on the
/// [`AssetServer`](bevy::asset::AssetServer) (`bevy-traps.md` #1), so a
/// `MinimalPlugins` headless app stays a no-op. On a live `.ron` edit the
/// generic redrive overwrites [`FxTuning`] through `ResMut`, so the projectile
/// and impact systems read the new size / velocity / stagger / impact timing
/// the very next frame — WITHOUT a rebuild.
pub(crate) fn register_fx_tuning_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<FxTuning>(FX_TUNING_RON_PATH);
}

#[cfg(test)]
mod test {
    use super::{
        FctRiseRate, FctTtlSeconds, FxTuning, ImpactFrameSeconds, InterShotSeconds,
        ProjectileDrawScale, ProjectileVelocity,
    };

    /// The shipped `fx.tuning.ron` parses into `FxTuning` and carries every tuning value —
    /// a `ron::de` round-trip of the SHIPPED bytes (a missing-but-required field would be a
    /// deserialize error; an absent field falls back to its `Default`).
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

    /// `FxTuning::default()` reproduces the user's CURRENT working-tree tune EXACTLY — the
    /// migration must not silently change a tuning value (the `0.5` draw scale, the `480`
    /// velocity, the `0.35` stagger, the `0.08` impact hold; the `DEFAULT` consts are the
    /// shipped values, so this guards the newtype `Default` impls against drift).
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

    /// A PARTIAL `.ron` (one field authored, the rest omitted) parses, taking the authored
    /// value and falling back to the shipped default for every omitted field — the
    /// `#[serde(default)]` contract that lets a user tune one number without restating the rest.
    #[test]
    fn partial_fx_tuning_ron_falls_back_to_defaults() {
        // Only the velocity is authored; the other three must fall back to their defaults.
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
