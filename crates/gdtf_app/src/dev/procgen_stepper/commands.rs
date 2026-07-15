//! The stepper UI's ONE-frame-idempotent command latch (bevy-traps #8b): a `bevy_egui`
//! multipass frame may run the panel closure TWICE to settle first-frame sizing, so every
//! button press is recorded via ASSIGNMENT (never a push/queue/counter) — latching the
//! SAME command twice in one frame leaves the SAME end state as latching it once, and a
//! separate `Update` system drains the latch exactly once per frame.

use std::time::Duration;

use bevy::prelude::*;

crate::support_item! {
    /// A single stepper action the egui panel can request — advance one stage, or drive
    /// every remaining stage to completion immediately.
    ///
    /// A named enum (no-bare-types: the panel's requested action is a domain value, never
    /// a bare bool/int). `pub` under `test-support` (the integration test drives the
    /// ENGAGED stepper by writing these directly — the egui closure never runs headlessly),
    /// `pub(crate)` otherwise.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum StepCommand {
        /// Advance exactly one stage.
        Next,
        /// Drive every remaining stage to completion immediately.
        Skip,
    }
}

crate::support_item! {
    /// The idempotent one-frame command latch.
    ///
    /// The egui panel calls [`request`](Self::request) (an ASSIGNMENT, not a push)
    /// whenever a button reads pressed this pass; [`super::drive::advance_stepper_drive`]
    /// (one `Update` system, so it runs exactly once per frame) [`take`](Self::take)s it
    /// and applies it. Because `request` assigns rather than queues, a multipass frame
    /// that calls it twice with the SAME command leaves the SAME latched value —
    /// idempotent by construction, never a double-advance from one user click.
    #[derive(Resource, Debug, Default, Clone, Copy)]
    struct PendingStepCommand(Option<StepCommand>);
}

impl PendingStepCommand {
    crate::support_item! {
        /// Latch `command` as pending (overwrites any prior pending command this frame).
        const fn request(&mut self, command: StepCommand) {
            self.0 = Some(command);
        }
    }

    crate::support_item! {
        /// Take (and clear) the pending command, if any.
        const fn take(&mut self) -> Option<StepCommand> {
            self.0.take()
        }
    }
}

crate::support_item! {
    /// Whether the stepper is free-running via Auto.
    ///
    /// Set by ASSIGNMENT every frame the egui panel draws (the checkbox's current read),
    /// idempotent under a multipass re-run for the same reason [`PendingStepCommand`] is:
    /// assigning the SAME bool twice in one frame has no second-order effect.
    #[derive(Resource, Debug, Default, Clone, Copy)]
    struct AutoRunning(bool);
}

impl AutoRunning {
    crate::support_item! {
        /// Set the Auto-run toggle to `running`.
        const fn set(&mut self, running: bool) {
            self.0 = running;
        }
    }

    crate::support_item! {
        /// Whether Auto-run is currently toggled on.
        #[must_use]
        const fn is_running(&self) -> bool {
            self.0
        }
    }
}

crate::support_item! {
    /// The fixed per-stage delay Auto-run waits between stages, so a developer watching the
    /// panel can actually see each stage land rather than it flashing past in one frame.
    ///
    /// `pub` under `test-support` (the GTW-655 integration test drives Auto free-run by
    /// stepping the app's virtual clock in increments of this exact delay, rather than
    /// re-hardcoding the shipped pace), `pub(crate)` otherwise.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct AutoStepDelay(Duration);
}

impl AutoStepDelay {
    crate::support_item! {
        /// The shipped Auto-run pace: one stage every 400 ms.
        const DEFAULT: Self = Self(Duration::from_millis(400));
    }

    crate::support_item! {
        /// The wrapped [`Duration`].
        #[must_use]
        const fn duration(self) -> Duration {
            self.0
        }
    }
}

/// The Auto-run pacing timer — repeating, [`AutoStepDelay::DEFAULT`] long. Ticked every
/// `Update` while [`AutoRunning::is_running`]; each time it finishes, one stage advances.
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

    /// Latching the SAME command twice (a multipass frame's two egui-closure runs,
    /// bevy-traps #8b) leaves the SAME end state as latching it once: exactly one command
    /// is there to `take`.
    #[test]
    fn request_called_twice_in_one_frame_is_idempotent() {
        let mut pending = PendingStepCommand::default();
        pending.request(StepCommand::Next);
        pending.request(StepCommand::Next);
        assert_eq!(pending.take(), Some(StepCommand::Next));
        assert_eq!(pending.take(), None);
    }

    /// A LATER request in the same frame overwrites an earlier one (still one end value,
    /// never a queue of two).
    #[test]
    fn request_called_twice_with_different_commands_keeps_only_the_latest() {
        let mut pending = PendingStepCommand::default();
        pending.request(StepCommand::Next);
        pending.request(StepCommand::Skip);
        assert_eq!(pending.take(), Some(StepCommand::Skip));
    }

    /// Setting the Auto-run toggle to the SAME value twice in one frame is a no-op the
    /// second time (an assignment, never a flip).
    #[test]
    fn auto_running_set_twice_in_one_frame_is_idempotent() {
        let mut auto = AutoRunning::default();
        assert!(!auto.is_running());
        auto.set(true);
        auto.set(true);
        assert!(auto.is_running());
    }
}
