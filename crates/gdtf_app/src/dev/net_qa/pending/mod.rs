//! The typed pending queues + the frame-deadline sweep (GTW-736).
//!
//! Requests the router cannot answer synchronously (an [`Inject`], a snapshot, an event
//! drain, a screenshot, a screenshot-after, a battle start, a stepper drive, a menu-item
//! activation) land in a per-kind [`PendingQueue`] where a LATER child (T4-T7 / T9 / T15 /
//! GTW-766 / GTW-787) consumes them. Every entry carries a frame-deadline countdown; the
//! [`sweep_pending`] pump answers the client with a
//! [`Timeout`](gdtf_qa_protocol::envelope::QaError::Timeout) when it expires unclaimed — so
//! a request never leaves the client hanging.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`payloads`] — the typed per-request payload structs (the arguments each consumer
//!   reads) and their manual `Debug` impls.
//! - [`queue`] — the [`PendingQueue`] FIFO + its frame-deadline sweep + the
//!   [`PendingQueues`] router bundle.

mod payloads;
mod queue;

pub(in crate::dev::net_qa) use payloads::{
    ActivateMenuPayload, InjectPayload, OutputPayload, ScreenshotAfterPayload, ScreenshotPayload,
    SnapshotPayload, StartBattlePayload, StepperControlPayload,
};
pub(in crate::dev::net_qa) use queue::{PendingQueue, PendingQueues, sweep_pending};
