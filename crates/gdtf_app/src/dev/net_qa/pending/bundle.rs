//! The [`PendingQueues`] router argument bundle (GTW-736; GTW-803 lifted the queue
//! machinery itself into `gdtf_net_qa_transport`, leaving this file its one concern).
//!
//! The queue type, its per-entry frame deadline and the sweep pump are the host-agnostic
//! transport's ([`PendingQueue`], [`sweep_pending`](gdtf_net_qa_transport::sweep_pending));
//! what stays here is THIS host's instantiation of them — one `ResMut` per battle payload
//! type, bundled so the router stays under the argument-count ceiling. The payload types
//! themselves live in the sibling [`payloads`](super::payloads) module.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_net_qa_transport::PendingQueue;

use super::payloads::{
    ActivateMenuPayload, FocusControlPayload, InjectPayload, OutputPayload, ScreenshotAfterPayload,
    ScreenshotPayload, SnapshotPayload, StartBattlePayload, StepperControlPayload,
};

/// The bundle of every typed pending queue the router enqueues into — one
/// [`SystemParam`] so the router stays under the argument-count ceiling.
#[derive(SystemParam)]
pub(in crate::dev::net_qa) struct PendingQueues<'w> {
    /// Injected intents (T4).
    pub(in crate::dev::net_qa) inject:           ResMut<'w, PendingQueue<InjectPayload>>,
    /// Battle-state snapshots (T5).
    pub(in crate::dev::net_qa) snapshot:         ResMut<'w, PendingQueue<SnapshotPayload>>,
    /// Event drains (T6).
    pub(in crate::dev::net_qa) output:           ResMut<'w, PendingQueue<OutputPayload>>,
    /// Screenshots (T7).
    pub(in crate::dev::net_qa) screenshot:       ResMut<'w, PendingQueue<ScreenshotPayload>>,
    /// Screenshot-after requests (T15) — battle-dependent (the embedded intent needs a
    /// live battle), gated at route time exactly like a bare `Inject`.
    pub(in crate::dev::net_qa) screenshot_after: ResMut<'w, PendingQueue<ScreenshotAfterPayload>>,
    /// Battle starts (T9).
    pub(in crate::dev::net_qa) start_battle:     ResMut<'w, PendingQueue<StartBattlePayload>>,
    /// DEV procgen stepper-drive commands (GTW-766) — gated at route time on a live
    /// `StagedProcgen` drive, not on a battle.
    pub(in crate::dev::net_qa) stepper_control:  ResMut<'w, PendingQueue<StepperControlPayload>>,
    /// Menu-item activations (GTW-787) — always serviceable at route time (the consumer
    /// validates the token against the live menu and answers a stale one).
    pub(in crate::dev::net_qa) activate_menu:    ResMut<'w, PendingQueue<ActivateMenuPayload>>,
    /// Focus-drive commands (GTW-802) — always serviceable at route time (the consumer
    /// validates the token against the live focus graph and answers a stale one), and
    /// NEVER battle-gated: a focus-navigable screen is usually off-battle.
    pub(in crate::dev::net_qa) focus_control:    ResMut<'w, PendingQueue<FocusControlPayload>>,
}
