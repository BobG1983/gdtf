use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use gdtf_battle_sim::procgen::StagedProcgen;

use super::{
    commands::{AutoRunning, PendingStepCommand, StepCommand},
    schematic::draw_schematic,
};

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
