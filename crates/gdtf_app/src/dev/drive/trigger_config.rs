//! The env config half of the DEV battle-script drive triggers: the fire / fall
//! trigger frames (`GDTF_FIRE_AT_FRAME` / `GDTF_FALL_AT_FRAME`) and the fire-mode
//! override parse (`GDTF_FIRE_MODE`). Split out of the capture plugin (GTW-583) and
//! homed with the drive affordance (GTW-632); see the capture plugin's header
//! (`crate::dev::capture::plugin`) for the full affordance rationale.

use bevy::prelude::*;
use gdtf_battle_sim::weapon::ModeKind;

/// The `GDTF_FIRE_AT_FRAME` environment variable: the `BattleRunning` frame at which the
/// selected player ganger fires at the nearest enemy via the real fire path (parsed into
/// a [`FireAtFrame`]). Read by the capture module's env-resolution seam
/// (`crate::dev::capture::resolve`), hence `pub(in crate::dev)`.
pub(in crate::dev) const FIRE_AT_FRAME_ENV: &str = "GDTF_FIRE_AT_FRAME";

/// The `GDTF_FIRE_MODE` environment variable: which fire MODE the dev fire-trigger shoots
/// in — `single` / `burst` / `full` (parsed into a [`FireModeOverride`]). Unset leaves the
/// trigger using the resident [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode). Used by the GTW-306 FX capture to drive
/// a multi-round (burst / full-auto) volley so the staggered projectiles are observable.
pub(in crate::dev) const FIRE_MODE_ENV: &str = "GDTF_FIRE_MODE";

/// The `GDTF_FALL_AT_FRAME` environment variable: the `BattleRunning` frame at which a
/// determinate player ganger is forced to FALL via the real GTW-523 fall path (parsed into
/// a [`FallAtFrame`]). Mirrors [`FIRE_AT_FRAME_ENV`] exactly — the fall counterpart of the
/// fire trigger, added (GTW-529) so the GTW-524 fall FX has a scripted in-engine QA trigger
/// (`GDTF_FIRE_AT_FRAME` only targets an enemy ganger, never a slab under a friendly).
pub(in crate::dev) const FALL_AT_FRAME_ENV: &str = "GDTF_FALL_AT_FRAME";

/// The frame at which the dev fire-trigger fires the selected player ganger.
///
/// A named newtype over `u32` (no-bare-types). `pub(crate)`: referenced only by the
/// in-crate wiring + config tests + the [`trigger_fire_at_frame`](super::triggers::trigger_fire_at_frame) system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref)]
pub(crate) struct FireAtFrame(u32);

impl FireAtFrame {
    /// Build a fire-trigger frame from its raw frame index. Test-only inherent surface
    /// (the production parse builds one through the tuple constructor in-module; the test
    /// constructs through the newtype, not the private field). `#[cfg(test)]` so the
    /// binary stays `dead_code`-clean.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn new(frame: u32) -> Self {
        Self(frame)
    }

    /// Parse a raw env-var value into an optional [`FireAtFrame`]: `Some` for a valid
    /// `u32`, `None` (trigger inert) for an absent / empty / non-numeric value. Pure —
    /// the env read + the loud set-but-invalid diagnostic live in the capture module's
    /// `resolve` seam (`crate::dev::capture::resolve`, GTW-590); never panics.
    #[must_use]
    pub(crate) fn parse(value: Option<&str>) -> Option<Self> {
        value
            .and_then(|raw| raw.trim().parse::<u32>().ok())
            .map(Self)
    }
}

/// The frame at which the dev FALL-trigger forces a determinate player ganger to fall.
///
/// A named newtype over `u32` (no-bare-types). Mirrors [`FireAtFrame`] exactly.
/// `pub(crate)`: referenced only by the in-crate wiring + config tests + the
/// [`trigger_fall_at_frame`](super::triggers::trigger_fall_at_frame) system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref)]
pub(crate) struct FallAtFrame(u32);

impl FallAtFrame {
    /// Build a fall-trigger frame from its raw frame index. Test-only inherent surface
    /// (the production parse builds one through the tuple constructor in-module; the test
    /// constructs through the newtype, not the private field). `#[cfg(test)]` so the binary
    /// stays `dead_code`-clean.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn new(frame: u32) -> Self {
        Self(frame)
    }

    /// Parse a raw env-var value into an optional [`FallAtFrame`]: `Some` for a valid
    /// `u32`, `None` (trigger inert) for an absent / empty / non-numeric value. Pure —
    /// the env read + the loud set-but-invalid diagnostic live in the capture module's
    /// `resolve` seam (`crate::dev::capture::resolve`, GTW-590); never panics. Mirrors
    /// [`FireAtFrame::parse`].
    #[must_use]
    pub(crate) fn parse(value: Option<&str>) -> Option<Self> {
        value
            .and_then(|raw| raw.trim().parse::<u32>().ok())
            .map(Self)
    }
}

/// An optional dev override for the fire-trigger's fire MODE — which authored
/// [`ModeKind`] the triggered shot fires in (`Single` / `Burst` / `Full`).
///
/// When set (`GDTF_FIRE_MODE`), [`trigger_fire_at_frame`](super::triggers::trigger_fire_at_frame) picks the matching
/// [`FireModeSpec`](gdtf_battle_sim::weapon::FireModeSpec) off the selected shooter's authored [`FireMode`](gdtf_battle_sim::weapon::FireMode) selector and fires in
/// THAT mode (so a `full` override produces a multi-round volley the FX stagger spreads
/// out). When unset the trigger uses the resident [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) unchanged.
///
/// A named newtype over the closed [`ModeKind`] (no-bare-types). `pub(crate)`: referenced
/// only by the in-crate wiring + config tests + [`trigger_fire_at_frame`](super::triggers::trigger_fire_at_frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref)]
pub(crate) struct FireModeOverride(ModeKind);

impl FireModeOverride {
    /// Build a fire-mode override from a [`ModeKind`]. Test-only inherent surface (the
    /// production path constructs it through [`FireModeOverride::parse`]); `#[cfg(test)]`
    /// so the binary stays `dead_code`-clean.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn new(kind: ModeKind) -> Self {
        Self(kind)
    }

    /// Parse a raw env-var value into an optional [`FireModeOverride`]: `Some` for a
    /// recognised, case-insensitive mode name (`single` / `burst` / `full` / `full-auto`),
    /// `None` for an absent / empty / unrecognised value. Pure — the env read + the loud
    /// set-but-unrecognised diagnostic live in the capture module's `resolve` seam
    /// (`crate::dev::capture::resolve`, GTW-590); never panics.
    #[must_use]
    pub(crate) fn parse(value: Option<&str>) -> Option<Self> {
        let kind = match value?.trim().to_ascii_lowercase().as_str() {
            "single" => ModeKind::Single,
            "burst" => ModeKind::Burst,
            "full" | "full-auto" | "fullauto" => ModeKind::Full,
            _ => return None,
        };
        Some(Self(kind))
    }
}

/// The resolved fire-trigger configuration: the frame to fire on.
///
/// Built by the capture module's `resolve` seam (`crate::dev::capture::resolve`), held
/// by [`DevCapturePlugin`](crate::dev::capture::plugin::DevCapturePlugin) when the fire
/// sub-affordance is enabled, inserted as a [`Resource`] so
/// [`trigger_fire_at_frame`](super::triggers::trigger_fire_at_frame) can read it.
/// `pub(crate)`: internal only. Fields are `pub(in crate::dev)` for exactly those two
/// capture-module consumers (the `resolve` struct literal + the plugin's activation log).
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct FireConfig {
    /// The `BattleRunning` frame at which the selected player ganger fires.
    pub(in crate::dev) frame: FireAtFrame,
    /// An optional fire-MODE override (`GDTF_FIRE_MODE`): when set, the trigger fires in
    /// this authored [`ModeKind`] (read off the shooter's [`FireMode`](gdtf_battle_sim::weapon::FireMode)) rather than the
    /// resident [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) — the FX-capture path uses `Full` for a staggered
    /// multi-round volley.
    pub(in crate::dev) mode:  Option<FireModeOverride>,
}

impl FireConfig {
    /// Build a fire-trigger config from the frame to fire on and an optional mode override.
    /// Test-only inherent surface (the production path constructs it via struct literal in
    /// the `resolve` module; the headless test seeds the REAL resource the
    /// [`trigger_fire_at_frame`](super::triggers::trigger_fire_at_frame) system reads
    /// through this). `#[cfg(test)]` so the binary stays `dead_code`-clean.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn new(frame: FireAtFrame, mode: Option<FireModeOverride>) -> Self {
        Self { frame, mode }
    }
}

/// The resolved fall-trigger configuration: the frame to force a fall on.
///
/// Built by the capture module's `resolve` seam (`crate::dev::capture::resolve`), held
/// by [`DevCapturePlugin`](crate::dev::capture::plugin::DevCapturePlugin) when the fall
/// sub-affordance is enabled, inserted as a [`Resource`] so
/// [`trigger_fall_at_frame`](super::triggers::trigger_fall_at_frame) can read it.
/// `pub(crate)`: internal only. Mirrors [`FireConfig`].
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct FallConfig {
    /// The `BattleRunning` frame at which the chosen player ganger is forced to fall.
    pub(in crate::dev) frame: FallAtFrame,
}

impl FallConfig {
    /// Build a fall-trigger config from the frame to force a fall on. Test-only inherent
    /// surface (the production path constructs it via struct literal in the `resolve`
    /// module; the headless test seeds the REAL resource the
    /// [`trigger_fall_at_frame`](super::triggers::trigger_fall_at_frame) system reads
    /// through this). `#[cfg(test)]` so the binary stays `dead_code`-clean.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn new(frame: FallAtFrame) -> Self {
        Self { frame }
    }
}
