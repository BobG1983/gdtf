pub(crate) const DEADLINE_BUDGET: FrameDeadline = FrameDeadline::new(4);

#[derive(Debug, Clone, Copy)]
pub(super) struct FrameDeadline(u32);

pub(super) enum DeadlineTick {
    Live,
    Expired,
}

impl FrameDeadline {
    const fn new(frames: u32) -> Self {
        Self(frames)
    }

    pub(super) const fn tick(&mut self) -> DeadlineTick {
        if self.0 == 0 {
            return DeadlineTick::Expired;
        }
        self.0 -= 1;
        DeadlineTick::Live
    }
}
