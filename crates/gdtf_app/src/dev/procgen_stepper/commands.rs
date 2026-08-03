//! Procgen stepper command resources.

use std::time::Duration;

use bevy::prelude::*;

crate::support_item! {
    /// Manual step command for the procgen stepper.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum StepCommand {
        /// Advance one stage.
        Next,
        /// Skip remaining stages.
        Skip,
    }
}

crate::support_item! {
    #[derive(Resource, Debug, Default, Clone, Copy)]
    struct PendingStepCommand(Option<StepCommand>);
}

impl PendingStepCommand {
    crate::support_item! {
        const fn request(&mut self, command: StepCommand) {
            self.0 = Some(command);
        }
    }

    crate::support_item! {
        const fn take(&mut self) -> Option<StepCommand> {
            self.0.take()
        }
    }
}

crate::support_item! {
    #[derive(Resource, Debug, Default, Clone, Copy)]
    struct AutoRunning(bool);
}

impl AutoRunning {
    crate::support_item! {
        const fn set(&mut self, running: bool) {
            self.0 = running;
        }
    }

    crate::support_item! {
        #[must_use]
        const fn is_running(&self) -> bool {
            self.0
        }
    }
}

crate::support_item! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct AutoStepDelay(Duration);
}

impl AutoStepDelay {
    crate::support_item! {
        const DEFAULT: Self = Self(Duration::from_millis(400));
    }

    crate::support_item! {
        #[must_use]
        const fn duration(self) -> Duration {
            self.0
        }
    }
}

#[derive(Resource, Debug, Deref, DerefMut)]
pub(crate) struct AutoStepTimer(Timer);

impl Default for AutoStepTimer {
    fn default() -> Self {
        Self(Timer::new(
            AutoStepDelay::DEFAULT.duration(),
            TimerMode::Repeating,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::{AutoRunning, PendingStepCommand, StepCommand};

    #[test]
    fn request_called_twice_in_one_frame_is_idempotent() {
        let mut pending = PendingStepCommand::default();
        pending.request(StepCommand::Next);
        pending.request(StepCommand::Next);
        assert_eq!(pending.take(), Some(StepCommand::Next));
        assert_eq!(pending.take(), None);
    }

    #[test]
    fn request_called_twice_with_different_commands_keeps_only_the_latest() {
        let mut pending = PendingStepCommand::default();
        pending.request(StepCommand::Next);
        pending.request(StepCommand::Skip);
        assert_eq!(pending.take(), Some(StepCommand::Skip));
    }

    #[test]
    fn auto_running_set_twice_in_one_frame_is_idempotent() {
        let mut auto = AutoRunning::default();
        assert!(!auto.is_running());
        auto.set(true);
        auto.set(true);
        assert!(auto.is_running());
    }
}
