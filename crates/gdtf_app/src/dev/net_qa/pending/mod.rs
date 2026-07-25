//! The game's typed pending-request payloads + the router's queue bundle (GTW-736).
//!
//! Requests the router cannot answer synchronously (an [`Inject`], a snapshot, an event
//! drain, a screenshot, a screenshot-after, a battle start, a stepper drive, a menu-item
//! activation, a focus drive) land in a per-kind
//! [`PendingQueue`](gdtf_net_qa_transport::PendingQueue) where a LATER child (T4-T7 / T9 /
//! T15 / GTW-766 / GTW-787 / GTW-802) consumes them. The queue itself, its frame-deadline
//! countdown and the [`sweep_pending`](gdtf_net_qa_transport::sweep_pending) pump that
//! answers an unclaimed entry are the host-agnostic transport's (GTW-803); what lives here
//! is what those queues carry FOR THIS HOST.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`payloads`] — the typed per-request payload structs (the arguments each consumer
//!   reads) and their manual `Debug` impls.
//! - [`bundle`] — the [`PendingQueues`] router argument bundle over those payloads.

mod bundle;
mod payloads;

pub(in crate::dev::net_qa) use bundle::PendingQueues;
pub(in crate::dev::net_qa) use payloads::{
    ActivateMenuPayload, FocusControlPayload, InjectPayload, OutputPayload, ScreenshotAfterPayload,
    ScreenshotPayload, SnapshotPayload, StartBattlePayload, StepperControlPayload,
};
