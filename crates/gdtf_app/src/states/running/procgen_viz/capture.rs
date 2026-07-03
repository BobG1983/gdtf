//! DEV-ONLY procgen-visualizer self-screenshot QA hook (GTW-434, C5; ported onto the
//! `gdtf_screenshot` primitives in GTW-577 C4).
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
//! ## How it drives + captures (GTW-577: the shared primitives)
//!
//! The per-scene ENV VAR and the per-scene DRIVE stay bespoke here (P9); the settle counter,
//! the `Screenshot` + `save_to_disk` spawn, and the exit poll delegate to the shared pieces:
//! [`parse_shot_path`] (the path gate), [`settle_then_capture`] (the settle-then-spawn, with
//! this scene's calibrated [`SettleFrames`]), and the game-side
//! [`poll_then_quit`](crate::states::running::capture_exit::poll_then_quit) (PNG-on-disk →
//! [`RunningState::Quit`], the shared cascade — NEVER a direct `AppExit`, the macOS winit
//! hang Bevy #23313). When active it (a) drives the menu into
//! [`RunningState::DebugProcgenVisualizer`] the moment the menu rests, (b) reveals the WHOLE
//! placement (the AUTO end state, driven on the model directly — see [`drive_capture_auto`]
//! for why a synthesized button press is defeated by `ui_focus_system`), then (c) the chained
//! shared systems settle, capture, and quit. The capture is gated on the visualizer root
//! being present, so it retries until the screen exists instead of silently skipping (the
//! GTW-577 retirement of the old `!= SETTLE_FRAMES` exact-match skip).

use bevy::prelude::*;
use gdtf_screenshot::{
    CapturePath, CaptureProgress, PollCap, SettleFrames, parse_shot_path, settle_then_capture,
};

use crate::states::{
    RunningState,
    running::{
        capture_exit::poll_then_quit,
        procgen_viz::{components::ProcgenVizRoot, model::ProcgenViz},
    },
};

/// The `GDTF_PROCGEN_VIZ_SCREEN_SHOT` env var: the absolute path of the output PNG. Setting it
/// (in a `dev_capture` debug build) opts into the visualizer capture hook.
const VIZ_SHOT_ENV: &str = "GDTF_PROCGEN_VIZ_SCREEN_SHOT";

/// This scene's calibrated settle window: 45 visualizer frames, so the reveal-sync + UI layout
/// have flushed and the screen is settled rather than mid-layout (the GTW-419 settle
/// precedent; the per-scene magnitude stays bespoke — GTW-577 P9).
const VIZ_SETTLE: SettleFrames = SettleFrames::new(45);

/// Whether the visualizer capture hook is enabled, and where it writes.
///
/// `Some(path)` when [`VIZ_SHOT_ENV`] is set to a non-empty (trimmed) value; `None` (the hook
/// stays inert) otherwise. Pure aside from the env read: the trim/empty gate is the shared
/// [`parse_shot_path`] (GTW-577 C4 — the ONE path gate, asserted once in `gdtf_screenshot`).
#[must_use]
pub(in crate::states::running::procgen_viz) fn viz_shot_path() -> Option<CapturePath> {
    parse_shot_path(std::env::var(VIZ_SHOT_ENV).ok().as_deref())
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

/// Register the visualizer capture hook IF its env-var gate is set.
///
/// Called by [`ProcgenVizScenePlugin`](super::plugin::ProcgenVizScenePlugin) only under
/// `cfg!(all(debug_assertions, feature = "dev_capture"))`. When [`viz_shot_path`] returns
/// `None` it registers nothing (the hook is fully inert).
///
/// The per-scene DRIVE ([`drive_capture_auto`]) is chained AHEAD of the shared
/// [`settle_then_capture`] → [`poll_then_quit`] tail (GTW-577 C4/C5). The capture is
/// additionally gated on the [`ProcgenVizRoot`] being present, so it never captures a blank
/// screen — and retries until the screen exists. NOTE: the shared [`CapturePath`] /
/// [`SettleFrames`] resources mean ONE scene-capture env var per run (the QA workflow's
/// existing shape — each scene's capture is a separate app run anyway).
pub(in crate::states::running::procgen_viz) fn register_viz_capture(app: &mut App) {
    let Some(path) = viz_shot_path() else {
        return;
    };
    info!("procgen-viz capture: ON (dev) -> {}", path.display());
    app.insert_resource(path)
        .insert_resource(VIZ_SETTLE)
        .insert_resource(PollCap::DEFAULT)
        .init_resource::<CaptureProgress>()
        .add_systems(
            Update,
            drive_into_viz
                .run_if(in_state(RunningState::Menu).and_then(resource_exists::<CapturePath>)),
        )
        .add_systems(
            Update,
            (
                drive_capture_auto.run_if(resource_exists::<ProcgenViz>),
                settle_then_capture.run_if(any_with_component::<ProcgenVizRoot>),
                poll_then_quit,
            )
                .chain()
                .run_if(
                    in_state(RunningState::DebugProcgenVisualizer)
                        .and_then(resource_exists::<CapturePath>),
                ),
        );
}

#[cfg(test)]
mod tests {
    use super::VIZ_SHOT_ENV;

    /// The thin per-scene pin (GTW-577 C6): the scene KEEPS its own env var — the QA-facing
    /// activation contract — while the trim/empty gate logic is asserted ONCE in
    /// `gdtf_screenshot` (`parse_shot_path`'s own tests), which `viz_shot_path` delegates to.
    #[test]
    fn env_var_name_is_the_scene_contract() {
        assert_eq!(VIZ_SHOT_ENV, "GDTF_PROCGEN_VIZ_SCREEN_SHOT");
    }
}
