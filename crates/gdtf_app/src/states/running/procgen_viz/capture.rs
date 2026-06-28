//! DEV-ONLY procgen-visualizer self-screenshot QA hook (GTW-434, C5).
//!
//! This is **not shipping behavior**. It exists so QA (or a coding agent) can drive the app
//! into the procgen visualizer, reveal the placement, and capture the rendered screen —
//! proving C5 visually, which the headless tests structurally cannot observe. It mirrors the
//! GTW-420 gang-editor capture / GTW-297 [`DevCapturePlugin`](crate::app::capture) discipline.
//!
//! ## Two gates, both must hold to activate
//!
//! 1. **Dev cfg.** Wired into [`ProcgenVizScenePlugin`](super::plugin::ProcgenVizScenePlugin)
//!    only under `cfg!(all(debug_assertions, feature = "dev_capture"))`; a release / default
//!    build never compiles it.
//! 2. **Opt-in env var.** Even when compiled in it is inert until
//!    `GDTF_PROCGEN_VIZ_SCREEN_SHOT=/abs/out.png` is set: with it unset the hook registers
//!    nothing.
//!
//! ## How it drives + captures
//!
//! When active it (a) drives the menu into [`RunningState::DebugProcgenVisualizer`] the moment
//! the menu rests (the only non-automatic transition from launch — the screen then spawns
//! `OnEnter`), (b) once in the visualizer it reveals the WHOLE placement (the AUTO end state,
//! driven on the model directly — see [`drive_capture_auto`] for why a synthesized button press
//! is defeated by `ui_focus_system`), so the green player + red enemy + neutral fill quads are
//! all visible over the dark board, (c) waits a settle so the reveal-sync has flushed each
//! quad's display, captures the screen, and (d) rides the shared shutdown cascade by setting
//! [`RunningState::Quit`] (NOT writing `AppExit` — the macOS winit hang, Bevy #23313).

use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
};

use crate::states::{
    RunningState,
    running::procgen_viz::{components::ProcgenVizRoot, model::ProcgenViz},
};

/// The `GDTF_PROCGEN_VIZ_SCREEN_SHOT` env var: the absolute path of the output PNG. Setting it
/// (in a `dev_capture` debug build) opts into the visualizer capture hook.
const VIZ_SHOT_ENV: &str = "GDTF_PROCGEN_VIZ_SCREEN_SHOT";

/// How many visualizer frames to wait before capturing, so the reveal-sync + UI layout have
/// flushed and the screen is settled rather than mid-layout (the GTW-419 settle precedent).
const SETTLE_FRAMES: u32 = 45;

/// Whether the visualizer capture hook is enabled, and where it writes.
///
/// `Some(path)` when [`VIZ_SHOT_ENV`] is set to a non-empty (trimmed) value; `None` (the hook
/// stays inert) otherwise. Pure (no `World`); delegates the gate to [`parse_viz_shot_path`] so
/// the config test can drive the SAME logic without mutating the process-global env var.
#[must_use]
pub(in crate::states::running::procgen_viz) fn viz_shot_path() -> Option<PathBuf> {
    parse_viz_shot_path(std::env::var(VIZ_SHOT_ENV).ok().as_deref())
}

/// Apply the path gate to a raw env-var value: `Some(path)` when non-empty (trimmed), `None`
/// (hook inert) when absent / empty / all-whitespace. The pure core of [`viz_shot_path`].
#[must_use]
fn parse_viz_shot_path(value: Option<&str>) -> Option<PathBuf> {
    value
        .map(|raw| raw.trim().to_owned())
        .filter(|trimmed| !trimmed.is_empty())
        .map(PathBuf::from)
}

/// The resolved visualizer-shot configuration: where to write the captured PNG. Framework
/// plumbing (a path), not a domain value.
#[derive(Resource, Debug, Clone)]
pub(in crate::states::running::procgen_viz) struct VizShotConfig {
    /// Absolute path of the output PNG, handed to
    /// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk).
    path: PathBuf,
}

impl VizShotConfig {
    /// Build the config from the resolved output path.
    pub(in crate::states::running::procgen_viz) const fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

/// Drives the menu into [`RunningState::DebugProcgenVisualizer`] once the menu rests (the only
/// non-automatic launch transition), so the capture run reaches the visualizer unattended.
/// Param-only (`bevy-traps.md` #7).
fn drive_into_viz(mut next: ResMut<NextState<RunningState>>) {
    next.set(RunningState::DebugProcgenVisualizer);
}

/// Reveals the WHOLE placement ONCE so the captured frame shows every tinted quad over the
/// dark board (C5) — the capture-run equivalent of an AUTO press.
///
/// This mutates the [`ProcgenViz`] model directly (`reveal_all`) rather than injecting the
/// AUTO button's [`Interaction::Pressed`]: under the windowed `DefaultPlugins`, the built-in
/// `ui_focus_system` (`PreUpdate`) clears any injected `Pressed` back to `None` BEFORE the
/// `Update`-schedule control reader (`auto_on_press`) observes it, so a synthesized press is
/// defeated and never reveals the quads (bevy-traps #6). Driving the model state directly is
/// the same end state the AUTO control produces, without racing `ui_focus_system` — and it
/// leaves the real STEP / AUTO controls' `Changed<Interaction>` contract untouched (this is a
/// dev-only QA driver, not the gameplay path). A [`Local<bool>`] fires it once; the resource
/// is guaranteed present by the [`resource_exists::<ProcgenViz>`] run condition. Param-only
/// (`bevy-traps.md` #7).
fn drive_capture_auto(mut model: ResMut<ProcgenViz>, mut revealed_once: Local<bool>) {
    if *revealed_once {
        return;
    }
    model.reveal_all();
    *revealed_once = true;
}

/// Captures the rendered visualizer frame to disk after a brief settle, then exits the app via
/// the shared shutdown cascade.
///
/// Its [`Local<u32>`] counter increments each visualizer frame; on the settle frame
/// ([`SETTLE_FRAMES`]) it spawns a [`Screenshot::primary_window`] entity with an observer that
/// saves the PNG synchronously and then sets [`RunningState::Quit`] (the shared cascade — NOT
/// `AppExit`, the macOS hang #23313). Guards on the visualizer root being present so it never
/// captures a blank screen. Param-only (`bevy-traps.md` #7).
///
/// The actual screenshot needs a real render device, so it CANNOT be headless-tested — it is
/// verified by RUNNING the app (QA), then `Read`ing the PNG.
fn capture_viz_screen(
    mut commands: Commands,
    config: Res<VizShotConfig>,
    screens: Query<(), With<ProcgenVizRoot>>,
    mut frames: Local<u32>,
) {
    *frames += 1;
    if *frames != SETTLE_FRAMES {
        return;
    }
    if screens.iter().next().is_none() {
        // No visualizer screen yet (theme absent at spawn) — skip rather than capture blank.
        return;
    }
    let path = config.path.clone();
    commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>, mut next: ResMut<NextState<RunningState>>| {
            save_to_disk(&path)(captured);
            next.set(RunningState::Quit);
        },
    );
}

/// Register the visualizer capture hook IF its env-var gate is set.
///
/// Called by [`ProcgenVizScenePlugin`](super::plugin::ProcgenVizScenePlugin) only under
/// `cfg!(all(debug_assertions, feature = "dev_capture"))`. When [`viz_shot_path`] returns
/// `None` it registers nothing (the hook is fully inert).
pub(in crate::states::running::procgen_viz) fn register_viz_capture(app: &mut App) {
    let Some(path) = viz_shot_path() else {
        return;
    };
    info!("procgen-viz capture: ON (dev) -> {}", path.display());
    app.insert_resource(VizShotConfig::new(path))
        .add_systems(
            Update,
            drive_into_viz
                .run_if(in_state(RunningState::Menu).and_then(resource_exists::<VizShotConfig>)),
        )
        .add_systems(
            Update,
            drive_capture_auto.run_if(
                in_state(RunningState::DebugProcgenVisualizer)
                    .and_then(resource_exists::<VizShotConfig>)
                    .and_then(resource_exists::<ProcgenViz>),
            ),
        )
        .add_systems(
            Update,
            capture_viz_screen.after(drive_capture_auto).run_if(
                in_state(RunningState::DebugProcgenVisualizer)
                    .and_then(resource_exists::<VizShotConfig>),
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::parse_viz_shot_path;

    #[test]
    fn path_gate_accepts_non_empty_and_rejects_blank() {
        assert!(parse_viz_shot_path(Some("/abs/out.png")).is_some());
        assert!(parse_viz_shot_path(Some("  /trim/out.png  ")).is_some());
        assert!(parse_viz_shot_path(Some("")).is_none());
        assert!(parse_viz_shot_path(Some("   ")).is_none());
        assert!(parse_viz_shot_path(None).is_none());
    }
}
