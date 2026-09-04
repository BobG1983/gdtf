//! The timer that advances the stepper without a manual command.

use bevy::{ecs::system::SystemParam, prelude::*};

use super::commands::{AutoRunning, AutoStepTimer};

/// The clock a running stepper advances on.
#[derive(SystemParam)]
pub(super) struct AutoStepClock<'w> {
    running: Option<Res<'w, AutoRunning>>,
    timer:   Option<ResMut<'w, AutoStepTimer>>,
    time:    Res<'w, Time>,
}

impl AutoStepClock<'_> {
    /// Tick the clock and say whether a stage is due this frame.
    pub(super) fn stage_due(&mut self) -> StageDue {
        let (Some(running), Some(timer)) = (self.running.as_deref(), self.timer.as_deref_mut())
        else {
            return StageDue::new(false);
        };
        if !running.is_running() {
            return StageDue::new(false);
        }
        timer.tick(self.time.delta());
        StageDue::new(timer.just_finished())
    }
}

/// Whether the auto-run clock says to advance a procgen stage this frame.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StageDue(bool);

impl StageDue {
    /// Wrap the clock answer.
    const fn new(due: bool) -> Self {
        Self(due)
    }
}
