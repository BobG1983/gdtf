//! [`QaRequest`] — the client-to-server request vocabulary (GTW-734).

use serde::{Deserialize, Serialize};

use super::{hello::ProtocolVersion, stepper::StepperCommandNet};
use crate::{
    ids::{EventCap, FrameDelay, SeedNet, ShotName, SituationRef},
    intent::NetIntent,
    view::RequestKindNet,
};

/// A single request a QA client sends the game's `net_qa` server — one request in, one
/// [`QaResponse`](crate::envelope::QaResponse) out.
///
/// The full request surface: the [`Hello`](Self::Hello) handshake, the two read
/// snapshots ([`GetAppFlow`](Self::GetAppFlow) / [`GetBattleState`](Self::GetBattleState)),
/// intent [`Inject`](Self::Inject)ion, a [`TakeScreenshot`](Self::TakeScreenshot), the
/// frame-exact [`ScreenshotAfter`](Self::ScreenshotAfter), the event
/// [`GetOutput`](Self::GetOutput) drain, and the T9 navigation
/// [`StartBattle`](Self::StartBattle) (defined now so T9 does not churn the envelope).
/// An independent serde enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QaRequest {
    /// Open the session — negotiate the protocol version.
    Hello(ProtocolVersion),
    /// Read the app-flow snapshot (where the app is in its lifecycle).
    GetAppFlow,
    /// Read the whole battle snapshot.
    GetBattleState,
    /// Inject one battle intent.
    Inject(NetIntent),
    /// Capture a screenshot, optionally under a caller-chosen file stem.
    TakeScreenshot {
        /// The screenshot's file stem, or `None` for a server-chosen name.
        name: Option<ShotName>,
    },
    /// Inject one battle intent, then capture a screenshot `frame_delay` frames after
    /// it queues — the frame-exact capture a request/response round-trip cannot land
    /// (GTW-749). A rejected intent takes NO capture; it answers with the same typed
    /// rejection a bare [`Inject`](Self::Inject) would.
    ScreenshotAfter {
        /// The intent to inject before counting down to the capture.
        intent:      NetIntent,
        /// How many frames to wait, after the intent queues, before capturing.
        frame_delay: FrameDelay,
        /// The screenshot's file stem, or `None` for a server-chosen name.
        name:        Option<ShotName>,
    },
    /// Drain buffered combat events, optionally capping how many.
    GetOutput {
        /// The maximum number of events to drain, or `None` for the whole outbox.
        max: Option<EventCap>,
    },
    /// Start a battle from a situation, optionally with a deterministic seed (the T9
    /// navigation surface).
    StartBattle {
        /// The situation to start.
        situation: SituationRef,
        /// The RNG seed to pin, or `None` for a server-chosen seed.
        seed:      Option<SeedNet>,
    },
    /// Drive the DEV procgen load-time stepper (Next / Auto / Skip) — routed to the
    /// stepper's command latch exactly as an egui button press is (GTW-766). Serviceable
    /// only while a drive is in flight (a live `StagedProcgen`); otherwise it is answered
    /// [`StepperInactive`](crate::envelope::QaError::StepperInactive), never a silent no-op
    /// or a panic.
    StepperControl(StepperCommandNet),
}

impl QaRequest {
    /// This request's [`RequestKindNet`] — its discriminant without the payload.
    ///
    /// The wildcard-free map that keeps [`RequestKindNet`] in lock-step with `QaRequest`:
    /// adding a `QaRequest` variant breaks this `match` until it (and `RequestKindNet`)
    /// gain a matching arm. The game's `net_qa` router reads it to decide, from one
    /// predicate, both which requests it accepts in the current state and which kinds
    /// [`AppFlowView::available`](crate::view::AppFlowView) advertises — so the two can
    /// never disagree.
    #[must_use]
    pub const fn kind(&self) -> RequestKindNet {
        match self {
            Self::Hello(_) => RequestKindNet::Hello,
            Self::GetAppFlow => RequestKindNet::GetAppFlow,
            Self::GetBattleState => RequestKindNet::GetBattleState,
            Self::Inject(_) => RequestKindNet::Inject,
            Self::TakeScreenshot { .. } => RequestKindNet::TakeScreenshot,
            Self::ScreenshotAfter { .. } => RequestKindNet::ScreenshotAfter,
            Self::GetOutput { .. } => RequestKindNet::GetOutput,
            Self::StartBattle { .. } => RequestKindNet::StartBattle,
            Self::StepperControl(_) => RequestKindNet::StepperControl,
        }
    }
}
