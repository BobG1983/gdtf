//! The [`ScreenshotCapturePlugin`] builder: the env-var capture-then-exit affordance for a plain
//! binary (no game state machine).
//!
//! This wires the generic settle-then-capture + poll-then-exit systems (from [`crate::trigger`])
//! for a binary that has no game-state machine to ride for its exit — the content editor's simplest
//! integration path, and the reference wiring a programmatic QA harness can also drive. A
//! game-state-machine consumer (the `gdtf_app` scenes) does NOT use this plugin — it delegates to
//! the crate PRIMITIVES ([`parse_shot_path`](crate::path::parse_shot_path),
//! [`settle_then_capture`](crate::trigger::settle_then_capture)) but keeps its own scene-scoped
//! drive + `RunningState::Quit` cascade exit.
//!
//! ## Inert by default
//!
//! Built via [`ScreenshotCapturePlugin::from_env`], the plugin reads its opt-in env var ONCE; when
//! it is unset `Plugin::build` registers NOTHING — indistinguishable from
//! absent. When set, it inserts the [`CapturePath`] / [`SettleFrames`] / [`PollCap`] resources +
//! [`CaptureProgress`] and registers the two `Update` systems. Because the RESET of the per-run
//! counters is tied to a scene's `OnEnter(S)` (which needs the concrete `States` type), the
//! plugin does NOT register `OnEnter` — the caller registers
//! [`reset_progress`](crate::trigger::reset_progress) on its own `OnEnter(S)` (matching the
//! established `EditorCapturePlugin` wiring). For a stateless binary the counters start at zero, so
//! no reset is needed at all.

use std::env;

use bevy::prelude::*;

use crate::{
    path::{CapturePath, parse_shot_path},
    settle::{PollCap, SettleFrames},
    trigger::{CaptureProgress, poll_then_exit, settle_then_capture},
};

/// The env-var capture-then-exit plugin — the reusable, scene-agnostic form of the boilerplate the
/// five in-tree capture hooks used to duplicate.
///
/// Constructed via [`from_env`](ScreenshotCapturePlugin::from_env): inert (registers nothing) when
/// its opt-in env var is unset; when set it captures the primary window after the settle window and
/// exits via [`AppExit`] once the PNG lands (or the poll cap elapses).
pub struct ScreenshotCapturePlugin {
    /// The resolved output path, or `None` when the opt-in env var was unset (plugin inert). Read
    /// once at construction.
    path:   Option<CapturePath>,
    /// How many frames to wait before capturing.
    settle: SettleFrames,
    /// How many frames to poll the disk for the PNG before giving up.
    poll:   PollCap,
}

impl ScreenshotCapturePlugin {
    /// Read `env_var` ONCE and build the plugin with the DEFAULT settle / poll windows
    /// ([`SettleFrames::default`] = egui-safe 30 frames, [`PollCap::default`]). When `env_var` is
    /// unset (or empty / all-whitespace) the plugin is inert; when set to a path the affordance
    /// captures the primary window and exits.
    ///
    /// The path gate delegates to [`parse_shot_path`], so a config test drives the SAME logic with
    /// an injected value (no process-global env mutation).
    #[must_use]
    pub fn from_env(env_var: &str) -> Self {
        Self {
            path:   parse_shot_path(env::var(env_var).ok().as_deref()),
            settle: SettleFrames::default(),
            poll:   PollCap::default(),
        }
    }

    /// Build the plugin from an explicit [`CapturePath`] (the programmatic-driver path a QA harness
    /// uses — it hands the exact path to capture to, rather than an env var), with the default
    /// settle / poll windows.
    #[must_use]
    pub fn with_path(path: CapturePath) -> Self {
        Self {
            path:   Some(path),
            settle: SettleFrames::default(),
            poll:   PollCap::default(),
        }
    }

    /// Override the settle window (the number of frames to wait before capturing). Chains after
    /// [`from_env`](ScreenshotCapturePlugin::from_env) / [`with_path`](ScreenshotCapturePlugin::with_path).
    #[must_use]
    pub const fn settle(mut self, settle: SettleFrames) -> Self {
        self.settle = settle;
        self
    }

    /// Override the disk-poll cap (the number of frames to poll for the PNG before giving up).
    #[must_use]
    pub const fn poll_cap(mut self, poll: PollCap) -> Self {
        self.poll = poll;
        self
    }

    /// Whether the affordance will activate on `build` (a path is configured). The config test's
    /// gate assertion, driving the REAL construction without launching an app.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.path.is_some()
    }
}

impl Plugin for ScreenshotCapturePlugin {
    fn build(&self, app: &mut App) {
        let Some(path) = self.path.clone() else {
            // Inert by default: no env var / no path -> no systems, no resources, normal launch.
            return;
        };
        info!("gdtf_screenshot: capture ON -> {}", path.display());
        app.insert_resource(path)
            .insert_resource(self.settle)
            .insert_resource(self.poll)
            .init_resource::<CaptureProgress>()
            .add_systems(
                Update,
                (settle_then_capture, poll_then_exit)
                    .chain()
                    .run_if(resource_exists::<CapturePath>),
            );
    }
}
