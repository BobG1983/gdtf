//! Frame budget an unclaimed pending request is swept after.

/// Frames an unclaimed pending request is held before the sweep answers it.
pub const DEADLINE_BUDGET: FrameDeadline = FrameDeadline::new(4);

/// Frames left before a pending request expires.
#[derive(Debug, Clone, Copy)]
pub struct FrameDeadline(u32);

pub(super) enum DeadlineTick {
    Live,
    Expired,
}

impl FrameDeadline {
    const fn new(frames: u32) -> Self {
        Self(frames)
    }

    /// Frames still left on the deadline.
    #[must_use]
    pub const fn frames(&self) -> u32 {
        self.0
    }

    pub(super) const fn tick(&mut self) -> DeadlineTick {
        if self.0 == 0 {
            return DeadlineTick::Expired;
        }
        self.0 -= 1;
        DeadlineTick::Live
    }
}
