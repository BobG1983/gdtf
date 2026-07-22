//! The stepper's egui panel: a schematic overview of the procgen cursor's placed footprints
//! (the map assembling piece by piece) plus the Next / Auto / Skip controls (GTW-732).
//!
//! Runs in [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass) (never `Update` —
//! bevy-traps #8), gated on the [`StagedProcgen`] resource existing (only true while the
//! stepper is engaged for the current `BattleScapeState::Generation` span). Every button press
//! LATCHES a command into [`PendingStepCommand`] / [`AutoRunning`] rather than stepping the
//! driver directly — the actual `advance` happens in
//! [`super::drive::advance_stepper_drive`] (`Update`), so a multipass re-run of this closure
//! (bevy-traps #8b) never double-advances: `request`/`set` are ASSIGNMENTS, and the Auto toggle
//! is two absolute Start/Stop buttons (never a checkbox flip) for the same reason.
//!
//! The schematic itself ([`super::schematic::draw_schematic`]) is a pure egui-painter overview
//! of the cursor's [`placed_footprints`](StagedProcgen::placed_footprints) — a vague grid
//! outline with one footprint box per placement — EXPLICITLY NOT the real terrain renderer.
//! The finished map still appears through the real presenter once
//! [`super::drive::finish_stepper_drive`] writes the emitted `Situation` into `SetupBattleRequested`.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use gdtf_battle_sim::procgen::StagedProcgen;

use super::{
    commands::{AutoRunning, PendingStepCommand, StepCommand},
    schematic::draw_schematic,
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
        let footprints = driver.placed_footprints();
        ui.label(format!("Stage: {:?}", driver.stage()));
        ui.label(format!("{} placement(s)", footprints.len()));
        draw_schematic(ui, driver.grid_size(), &footprints);
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
