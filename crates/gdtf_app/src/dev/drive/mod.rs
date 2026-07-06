//! The DEV-ONLY battle-script DRIVE triggers (GTW-306 fire / GTW-529 fall; homed
//! here by GTW-632): env-scripted one-shot triggers that drive the REAL sim paths
//! at a configured `BattleRunning` frame, so an unattended QA capture records live
//! combat (a real volley's FX, a real fall's drop + impact flash).
//!
//! These are DRIVE affordances, not screenshot logic — the sibling `capture`
//! module (`crate::dev::capture`) owns the screenshot path. The two stay coupled
//! at exactly two seams: the capture module's `resolve` seam resolves the trigger
//! env vars alongside the capture ones (one snapshot, one loud-diagnostics pass —
//! GTW-590), and its `DevCapturePlugin` registers the trigger systems beside the
//! capture system (the affordances aggregate under one plugin owner).
//!
//! - `trigger_config` — the env vocabulary (`GDTF_FIRE_AT_FRAME` / `GDTF_FIRE_MODE`
//!   / `GDTF_FALL_AT_FRAME`), the pure parses, and the resolved `FireConfig` /
//!   `FallConfig` resources.
//! - `triggers` — the one-shot `Update` systems (`trigger_fire_at_frame` /
//!   `trigger_fall_at_frame`).

pub(in crate::dev) mod trigger_config;
pub(in crate::dev) mod triggers;

#[cfg(test)]
mod test;
