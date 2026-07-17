//! [`QaRequest`] — the client-to-server request vocabulary (GTW-734).

use serde::{Deserialize, Serialize};

use super::hello::ProtocolVersion;
use crate::{
    ids::{EventCap, SeedNet, ShotName, SituationRef},
    intent::NetIntent,
    view::RequestKindNet,
};

/// A single request a QA client sends the game's `net_qa` server — one request in, one
/// [`QaResponse`](crate::envelope::QaResponse) out.
///
/// The full request surface: the [`Hello`](Self::Hello) handshake, the two read
/// snapshots ([`GetAppFlow`](Self::GetAppFlow) / [`GetBattleState`](Self::GetBattleState)),
/// intent [`Inject`](Self::Inject)ion, a [`TakeScreenshot`](Self::TakeScreenshot), the
/// event [`GetOutput`](Self::GetOutput) drain, and the T9 navigation
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
            Self::GetOutput { .. } => RequestKindNet::GetOutput,
            Self::StartBattle { .. } => RequestKindNet::StartBattle,
        }
    }
}
