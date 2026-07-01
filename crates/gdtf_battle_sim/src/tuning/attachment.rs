//! The GTW-542 **weapon-attachment** tuning group — the magnitudes the NO-PAYLOAD
//! attachment tags ([`FastReload`](crate::weapon::AttachTag::FastReload) /
//! [`WhisperBore`](crate::weapon::AttachTag::WhisperBore)) read at the spawn-side
//! folder-function seam.
//!
//! Most GTW-542 attachment effects carry their magnitude as a NAMED-newtype payload on
//! the [`AttachTag`](crate::weapon::AttachTag) variant itself (so a scenario can author
//! a distinct value per fitting). The two tags with NO payload field —
//! [`FastReload`](crate::weapon::AttachTag::FastReload) and
//! [`WhisperBore`](crate::weapon::AttachTag::WhisperBore) — read their magnitude from
//! THIS tuning group instead, so the value stays data-driven (RON-authored, hot-tunable)
//! rather than hardcoded in the folder-fn.

use bevy::prelude::Deref;
use serde::Deserialize;

/// A **reload-time factor** — a multiplier applied to a weapon's
/// [`ReloadTu`](crate::magazine::ReloadTu) at spawn by a reload-speeding attachment
/// (GTW-542). A factor `< 1.0` speeds the reload (fewer TU); `1.0` is the identity.
///
/// A tuning COEFFICIENT (a dimensionless multiplier over the per-weapon reload cost),
/// shared by [`FastReload`](crate::weapon::AttachTag::FastReload) (which reads it from
/// [`AttachmentTuning`]) and the payload of
/// [`SumpSlickAction`](crate::weapon::AttachTag::SumpSlickAction) (a smaller speed-up).
/// Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON scalar.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReloadFactor(f32);

impl ReloadFactor {
    /// Build a reload-time factor from its multiplier magnitude (`< 1.0` speeds the
    /// reload; `1.0` is the identity).
    #[must_use]
    pub const fn new(factor: f32) -> Self {
        Self(factor)
    }
}

/// A **spread penalty** — extra angular dispersion (radians) an attachment ADDS to a
/// weapon's [`BaseSpread`](crate::weapon::BaseSpread) at spawn (GTW-542). A positive
/// value WIDENS the base cone (a jury-rigged, faster-cycling action trades accuracy for
/// speed).
///
/// A tuning COEFFICIENT (radians, the `base_spread` unit — angular, no pixel), the payload
/// of [`SumpSlickAction`](crate::weapon::AttachTag::SumpSlickAction). Private inner +
/// derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct SpreadPenalty(f32);

impl SpreadPenalty {
    /// Build a spread penalty from its angular magnitude (radians ADDED to base spread).
    #[must_use]
    pub const fn new(radians: f32) -> Self {
        Self(radians)
    }
}

/// A **reload-time delta** — extra Time Units an attachment ADDS to a weapon's
/// [`ReloadTu`](crate::magazine::ReloadTu) at spawn (GTW-542). A positive value makes the
/// reload SLOWER (a bulkier drum takes longer to swap).
///
/// A tuning COEFFICIENT (Time Units, matching [`ReloadTu`](crate::magazine::ReloadTu)'s
/// `u8` inner), the payload of
/// [`GoreSumpDrum`](crate::weapon::AttachTag::GoreSumpDrum). Private inner + derived
/// [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct ReloadDelta(u8);

impl ReloadDelta {
    /// Build a reload-time delta from its flat Time-Unit magnitude (ADDED to the
    /// per-weapon reload cost).
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

/// A **brace bonus** — the §1a stability-score points a bracing attachment adds
/// (GTW-542), the payload of
/// [`DeadmansBrace`](crate::weapon::AttachTag::DeadmansBrace). Fed as an additive
/// [`SightStability`](crate::stability::SightStability) contribution (the same seam a
/// [`Scoped`](crate::weapon::Scoped) optic uses), so a braced weapon aims steadier → a
/// tighter cone.
///
/// A tuning COEFFICIENT (stability-score points, the [`SightStabilityBonus`](crate::tuning::SightStabilityBonus)
/// unit). Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BraceBonus(f32);

impl BraceBonus {
    /// Build a brace bonus from its stability-score point magnitude (a positive steadier).
    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }
}

/// A **cone-mult delta** — the fraction an attachment ADDS to a fire mode's
/// [`ModeConeMult`](crate::weapon::ModeConeMult) at spawn (GTW-542), the payload of
/// [`ExecutionersChoke`](crate::weapon::AttachTag::ExecutionersChoke). A positive value
/// WIDENS the cone (a brutal choke trades precision for lethality).
///
/// A tuning COEFFICIENT (a dimensionless cone-multiplier addend, the
/// [`ModeConeMult`](crate::weapon::ModeConeMult) unit). Private inner + derived
/// [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ConeMultDelta(f32);

impl ConeMultDelta {
    /// Build a cone-mult delta from its dimensionless magnitude (ADDED to a mode's cone
    /// multiplier).
    #[must_use]
    pub const fn new(delta: f32) -> Self {
        Self(delta)
    }
}

/// The GTW-542 weapon-attachment tuning group — the magnitudes the NO-PAYLOAD attachment
/// tags read at the spawn-side folder-function seam.
///
/// A field of [`CombatTuning`](crate::tuning::CombatTuning); authored in
/// `assets/core_tuning/combat.tuning.ron` under `attachment:`. Only the tags WITHOUT a
/// payload field read this group ([`FastReload`](crate::weapon::AttachTag::FastReload) /
/// [`WhisperBore`](crate::weapon::AttachTag::WhisperBore)); every other tag carries its
/// magnitude as a named-newtype payload on the variant. Every leaf is a tunable balance
/// starting point — tests assert only the RELATIVE direction (faster / steadier), never
/// the magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct AttachmentTuning {
    /// The [`FastReload`](crate::weapon::AttachTag::FastReload) reload-time factor — the
    /// multiplier the fast-reload attachment applies to the weapon's reload cost (`< 1.0`
    /// speeds it).
    pub fast_reload_factor: ReloadFactor,
    /// The [`WhisperBore`](crate::weapon::AttachTag::WhisperBore) sight-stability bonus —
    /// the small §1a stability-score points the whisper-bore attachment adds (a suppressor
    /// bundled with a modest steadying), fed as a [`SightStability`](crate::stability::SightStability)
    /// contribution.
    pub whisper_bore_sight: BraceBonus,
}

impl Default for AttachmentTuning {
    fn default() -> Self {
        // Defensible-but-arbitrary starting points (tunable balance data, value-agnostic
        // tests only): a fast reload cuts the reload cost roughly in half; a whisper-bore
        // adds a modest steadying (smaller than the +15 full sight bonus).
        Self {
            fast_reload_factor: ReloadFactor::new(0.5),
            whisper_bore_sight: BraceBonus::new(8.0),
        }
    }
}
