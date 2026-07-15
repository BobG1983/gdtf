//! The stepper's egui panel: shows which stage the driver is on / just ran, a short
//! human-readable summary of what that stage produced, and the Next / Auto / Skip controls.
//!
//! Runs in [`EguiPrimaryContextPass`] (never `Update` — bevy-traps #8), gated on the
//! [`StagedProcgen`] resource existing (only true while the stepper is engaged for the
//! current `BattleScapeState::Generation` span). Every button press LATCHES a command into
//! [`PendingStepCommand`] / [`AutoRunning`] rather than stepping the driver directly — the
//! actual `advance` call happens in [`super::drive::advance_stepper_drive`] (`Update`), so a
//! multipass re-run of this closure (bevy-traps #8b) never double-advances: `request`/`set`
//! are ASSIGNMENTS, and the Auto toggle is modelled as two absolute Start/Stop buttons
//! (never a checkbox flip) for the same reason — an assignment made twice in one frame ends
//! at the SAME value; a flip made twice cancels itself out.
//!
//! Drawing intermediate procgen state through the REAL presenter is genuinely impractical
//! for the assemble/fill stages: they produce abstract placement data
//! ([`Placement`](gdtf_battle_sim::procgen::Placement) /
//! [`FilledPlacement`](gdtf_battle_sim::procgen::FilledPlacement)), not a
//! [`Situation`](gdtf_battle_sim::situation::Situation) the presenter's terrain draw reads —
//! so those two stages show a text summary here instead. The emit stage IS drawable: it
//! produces the full `Situation`, and [`super::drive::finish_stepper_drive`] writes it into
//! the SAME `SetupBattleRequested` the normal path writes, so the map appears through the
//! real presenter at that point exactly as it always has.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use gdtf_battle_sim::procgen::StagedProcgen;

use super::{
    commands::{AutoRunning, PendingStepCommand, StepCommand},
    summary::stage_summary,
};

/// Draws the stepper panel and latches any button press. `run_if(resource_exists::<StagedProcgen>)`
/// (registered by [`super::plugin::ProcgenStepperPlugin`]), so it draws only while a drive is
/// in flight for the current `BattleScapeState::Generation` span.
///
/// Returns `Result` so a missing primary egui context (`ctx_mut()?`) is handled, never
/// unwrapped (bevy-traps #8).
pub(super) fn draw_stepper_panel(
    mut contexts: EguiContexts,
    driver: Option<Res<StagedProcgen>>,
    pending: Option<ResMut<PendingStepCommand>>,
    auto_running: Option<ResMut<AutoRunning>>,
) -> Result {
    let (Some(driver), Some(mut pending), Some(mut auto_running)) = (driver, pending, auto_running)
    else {
        return Ok(());
    };
    let ctx = contexts.ctx_mut()?;
    egui::Window::new("Procgen Stepper").show(ctx, |ui| {
        ui.label(format!("Stage: {:?}", driver.stage()));
        ui.label(stage_summary(&driver));
        ui.horizontal(|ui| {
            if ui.button("Next").clicked() {
                pending.request(StepCommand::Next);
            }
            if auto_running.is_running() {
                if ui.button("Stop Auto").clicked() {
                    auto_running.set(false);
                }
            } else if ui.button("Start Auto").clicked() {
                auto_running.set(true);
            }
            if ui.button("Skip").clicked() {
                pending.request(StepCommand::Skip);
            }
        });
    });
    Ok(())
}
