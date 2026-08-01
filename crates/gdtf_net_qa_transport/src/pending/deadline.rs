//! The timeout policy for a pending request: the frames-of-grace budget and the per-entry
//! countdown (GTW-736).

/// The grace, in frames, a pending request is given before the sweep times it out.
///
/// A real consumer picks its queue entry up within a frame or two; this budget only bounds
/// how long an UNCLAIMED request waits before the client gets a prompt
/// [`Timeout`](gdtf_qa_protocol::message::QaError::Timeout) instead of a hang.
pub(crate) const DEADLINE_BUDGET: FrameDeadline = FrameDeadline::new(4);

/// A frames-remaining countdown after which an unclaimed pending request times out.
///
/// Private-inner newtype over `u32` (no-bare-types): the sweep [`tick`](Self::tick)s it
/// once per frame and answers [`Timeout`](gdtf_qa_protocol::message::QaError::Timeout)
/// on the tick it reaches zero.
#[derive(Debug, Clone, Copy)]
pub(super) struct FrameDeadline(u32);

/// The outcome of ticking a [`FrameDeadline`] — a typed alternative to a bare `bool`.
pub(super) enum DeadlineTick {
    /// Still within budget this frame.
    Live,
    /// The budget is spent — answer the client with a timeout.
    Expired,
}

impl FrameDeadline {
    /// Build a countdown of `frames` remaining.
    const fn new(frames: u32) -> Self {
        Self(frames)
    }

    /// Spend one frame of the budget, reporting whether it is now exhausted.
    pub(super) const fn tick(&mut self) -> DeadlineTick {
        if self.0 == 0 {
            return DeadlineTick::Expired;
        }
        self.0 -= 1;
        DeadlineTick::Live
    }
}
