//! DEV-ONLY loading-screen self-screenshot QA hook (GTW-419).
//!
//! This is **not shipping behavior**. It exists so QA (or a coding agent) can drive the app and
//! capture the rendered LOADING SCREEN frame — proving AC2 (no partial-level frame) visually,
//! which the headless tests structurally cannot observe. It mirrors the GTW-297
//! [`DevCapturePlugin`](crate::app::capture) gating discipline but fires DURING
//! [`BattleScapeState::Generation`](crate::states::BattleScapeState) rather than `BattleRunning`.
//!
//! ## Two gates, both must hold to activate
//!
//! 1. **Dev cfg.** Wired into [`LoadingScreenPlugin`](super::plugin::LoadingScreenPlugin) only
//!    under `cfg!(all(debug_assertions, feature = "dev_capture"))`; a release / default build
//!    never compiles it in.
//! 2. **Opt-in env var.** Even when compiled in it is inert until `GDTF_LOADING_SHOT=/abs/out.png`
//!    is set: with it unset the hook registers nothing.
//!
//! ## Holding the (brief) Generation state
//!
//! Generation auto-advances to `AnimateIn` the moment the sim signals `BattleReady` (a frame or
//! two). So that the screenshot reliably catches the LOADING SCREEN — not the assembled level or
//! a partial frame — the hook, while active, also INHIBITS the `Generation → AnimateIn`
//! transition until the capture has been taken (it re-pins `NextState` back to `Generation`),
//! then releases the pin AFTER the shot so the normal flow resumes. The pin is a dev-only QA
//! affordance: it never runs in a normal build.

use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
};

use crate::states::{
    BattleScapeState,
    running::game::battlescape::generation::loading_screen::components::LoadingScreenRoot,
};

/// The `GDTF_LOADING_SHOT` env var: the absolute path of the output PNG. Setting it (in a
/// `dev_capture` debug build) opts into the loading-screen capture hook.
const LOADING_SHOT_ENV: &str = "GDTF_LOADING_SHOT";

/// How many `Generation` frames to wait before capturing, so the UI layout has flushed and the
/// loading screen is settled rather than mid-layout (the GTW-297 settle precedent).
const SETTLE_FRAMES: u32 = 4;

/// Whether the loading-screen capture hook is enabled, and where it writes.
///
/// `Some(path)` when [`LOADING_SHOT_ENV`] is set to a non-empty (trimmed) value; `None` (the hook
/// stays inert) otherwise. Held by the hook resource so [`capture_loading_screen`] knows the
/// output path. The path is framework plumbing handed straight to
/// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk) — not a domain value —
/// so the no-bare-types rule does not apply to it.
///
/// Pure (no `World`) so the GUI path can be reasoned about without launching; delegates the
/// gate to [`parse_loading_shot_path`] so a future config test can drive the SAME logic without
/// mutating the process-global env var.
#[must_use]
pub(in crate::states::running::game::battlescape::generation) fn loading_shot_path()
-> Option<PathBuf> {
    parse_loading_shot_path(std::env::var(LOADING_SHOT_ENV).ok().as_deref())
}

/// Apply the path gate to a raw env-var value: `Some(path)` when non-empty (trimmed), `None`
/// (hook inert) when absent / empty / all-whitespace. The pure core of [`loading_shot_path`].
#[must_use]
fn parse_loading_shot_path(value: Option<&str>) -> Option<PathBuf> {
    value
        .map(|raw| raw.trim().to_owned())
        .filter(|trimmed| !trimmed.is_empty())
        .map(PathBuf::from)
}

/// The resolved loading-shot configuration: where to write the captured PNG.
///
/// Inserted as a [`Resource`] when the hook is enabled, so [`capture_loading_screen`] can read it.
/// Framework-plumbing config (a path), not a domain value.
#[derive(Resource, Debug, Clone)]
pub(in crate::states::running::game::battlescape::generation) struct LoadingShotConfig {
    /// Absolute path of the output PNG, handed to
    /// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk).
    path: PathBuf,
}

impl LoadingShotConfig {
    /// Build the config from the resolved output path.
    pub(in crate::states::running::game::battlescape::generation) const fn new(
        path: PathBuf,
    ) -> Self {
        Self { path }
    }
}

/// Captures the rendered loading-screen frame to disk after a brief settle, while PINNING the
/// `Generation` state so the shot catches the LOADING SCREEN (not the assembled level).
///
/// Runs in `Update`, gated `run_if(in_state(BattleScapeState::Generation))` AND on the config
/// resource existing. Its [`Local<u32>`] counter increments each Generation frame; until it has
/// settled ([`SETTLE_FRAMES`]) it re-pins [`NextState`] back to `Generation` (inhibiting the
/// `BattleReady`-gated `move_on`) so the loading screen stays on screen long enough to capture.
/// On the settle frame it spawns a [`Screenshot::primary_window`] entity with an observer that
/// saves the PNG synchronously, then RELEASES the pin (does not re-pin) so the normal
/// `Generation → AnimateIn` flow resumes on the next ready frame. A later Generation re-entry
/// (a second battle) would re-fire via the `Local` — acceptable dev tooling.
///
/// The actual screenshot needs a real render device, so it CANNOT be headless-tested — it is
/// verified by RUNNING the app (QA), then `Read`ing the PNG. Param-only (`bevy-traps.md` #7).
fn capture_loading_screen(
    mut commands: Commands,
    config: Res<LoadingShotConfig>,
    mut next: ResMut<NextState<BattleScapeState>>,
    screens: Query<(), With<LoadingScreenRoot>>,
    mut frames: Local<u32>,
) {
    *frames += 1;
    if *frames < SETTLE_FRAMES {
        // Hold the loading screen on screen: re-pin Generation so the BattleReady-gated move_on
        // cannot advance before the capture (the loading screen would otherwise vanish).
        next.set(BattleScapeState::Generation);
        return;
    }
    if *frames > SETTLE_FRAMES {
        // Already captured on the settle frame; release the pin (do nothing) so normal flow runs.
        return;
    }
    // The settle frame, AND the loading screen is actually present — capture it. (Guarding on the
    // root presence avoids a blank capture if the theme was absent and nothing spawned.)
    if screens.iter().next().is_none() {
        return;
    }
    let path = config.path.clone();
    commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>| {
            // Flush the PNG synchronously so the loading-screen frame is written to disk.
            save_to_disk(&path)(captured);
        },
    );
    // Do NOT re-pin this frame: the pin is released, so the next BattleReady frame advances
    // Generation → AnimateIn normally.
}

/// Register the loading-screen capture hook IF its env-var gate is set.
///
/// Called by [`LoadingScreenPlugin`](super::plugin::LoadingScreenPlugin) only under
/// `cfg!(all(debug_assertions, feature = "dev_capture"))`. When [`loading_shot_path`] returns
/// `None` it registers nothing (the hook is fully inert, exactly like a build without it).
pub(in crate::states::running::game::battlescape::generation) fn register_loading_capture(
    app: &mut App,
) {
    let Some(path) = loading_shot_path() else {
        return;
    };
    info!("loading-screen capture: ON (dev) -> {}", path.display());
    app.insert_resource(LoadingShotConfig::new(path))
        .add_systems(
            Update,
            capture_loading_screen.run_if(
                in_state(BattleScapeState::Generation)
                    .and_then(resource_exists::<LoadingShotConfig>),
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::parse_loading_shot_path;

    #[test]
    fn path_gate_accepts_non_empty_and_rejects_blank() {
        assert!(parse_loading_shot_path(Some("/abs/out.png")).is_some());
        assert!(parse_loading_shot_path(Some("  /trim/out.png  ")).is_some());
        assert!(parse_loading_shot_path(Some("")).is_none());
        assert!(parse_loading_shot_path(Some("   ")).is_none());
        assert!(parse_loading_shot_path(None).is_none());
    }
}
