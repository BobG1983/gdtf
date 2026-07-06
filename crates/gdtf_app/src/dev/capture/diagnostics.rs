//! The GTW-590 loud-diagnostics helpers of the DEV-ONLY capture affordance — the one
//! emission choke point for the resolved-config `warn!` lines and the capture
//! output-directory materialization whose failure `build()` logs loudly. Split out of
//! the sibling `plugin` module at its natural seam (GTW-632): these change with the
//! loudness contract, not with the plugin's registration wiring.

use bevy::prelude::*;

use super::{capture_config::CaptureConfig, resolve::ResolvedCaptureEnv};

/// `warn!` every [`CaptureConfigWarning`](super::resolve::CaptureConfigWarning)
/// diagnostic a resolved env snapshot carries, one `dev-capture: `-prefixed line per
/// warning — the ONE emission choke point for the GTW-590 loud-config lines (C4c).
///
/// [`DevCapturePlugin::from_env`](super::plugin::DevCapturePlugin::from_env) runs it on
/// the live snapshot; it takes the RESOLVED snapshot (rather than reading the env
/// itself) so the loudness test drives the REAL emitter with an injected snapshot under
/// the log capture, no process-global env mutation — deleting the `warn!` turns the
/// suite red instead of turning a misconfigured QA run silent. `pub(super)` for exactly
/// that test.
pub(super) fn warn_config_diagnostics(resolved: &ResolvedCaptureEnv) {
    for warning in &resolved.warnings {
        warn!("dev-capture: {warning}");
    }
}

/// Create the capture output's parent directory (`create_dir_all`) so every scheduled
/// PNG can land — GTW-590 C3's "parent dirs created or a loud error". `Ok` for a bare
/// filename (no parent) or an already-existing directory; `Err` carries the IO cause,
/// which [`DevCapturePlugin::build`](super::plugin::DevCapturePlugin) logs as an
/// immediate `error!` naming the output path. `pub(super)` so the schedule test pins
/// both the materialization and the failure classification.
pub(super) fn ensure_output_dir(config: &CaptureConfig) -> std::io::Result<()> {
    let Some(parent) = config.path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        // A bare relative filename ("shot.png"): nothing to create.
        return Ok(());
    }
    std::fs::create_dir_all(parent)
}
