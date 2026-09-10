use bevy::prelude::*;
use gdtf_battle_sim::procgen::ProcgenAdvance;

use super::{
    clock::AutoStepClock,
    commands::{AutoRunning, AutoStepTimer, PendingStepCommand, StepCommand},
};

pub(super) fn engage_stepper(mut commands: Commands) {
    commands.insert_resource(ProcgenAdvance::Hold);
    commands.insert_resource(PendingStepCommand::default());
    commands.insert_resource(AutoRunning::default());
    commands.insert_resource(AutoStepTimer::default());
}

pub(super) fn signal_stepper_advance(
    advance: Option<ResMut<ProcgenAdvance>>,
    pending: Option<ResMut<PendingStepCommand>>,
    mut clock: AutoStepClock,
) {
    let Some(mut advance) = advance else {
        return;
    };
    if let Some(command) = pending.and_then(|mut pending| pending.take()) {
        *advance = match command {
            StepCommand::Next => ProcgenAdvance::OneStage,
            StepCommand::Skip => ProcgenAdvance::AllStages,
        };
        return;
    }
    if *clock.stage_due() {
        *advance = ProcgenAdvance::OneStage;
    }
}

pub(super) fn cleanup_stepper_drive(mut commands: Commands) {
    commands.remove_resource::<ProcgenAdvance>();
    commands.remove_resource::<PendingStepCommand>();
    commands.remove_resource::<AutoRunning>();
    commands.remove_resource::<AutoStepTimer>();
}
