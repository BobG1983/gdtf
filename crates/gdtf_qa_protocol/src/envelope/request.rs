//! [`QaRequest`] — the client-to-server request vocabulary (GTW-734).

use serde::{Deserialize, Serialize};

use super::{focus::FocusCommandNet, hello::ProtocolVersion, stepper::StepperCommandNet};
use crate::{
    ids::{EventCap, FocusTargetNet, FrameDelay, SeedNet, ShotName, SituationRef},
    intent::NetIntent,
    view::{EditorQueryKind, RequestKindNet},
};

/// A single request a QA client sends the game's `net_qa` server — one request in, one
/// [`QaResponse`](crate::envelope::QaResponse) out.
///
/// The full request surface: the [`Hello`](Self::Hello) handshake, the two read
/// snapshots ([`GetAppFlow`](Self::GetAppFlow) / [`GetBattleState`](Self::GetBattleState)),
/// intent [`Inject`](Self::Inject)ion, a [`TakeScreenshot`](Self::TakeScreenshot), the
/// frame-exact [`ScreenshotAfter`](Self::ScreenshotAfter), the event
/// [`GetOutput`](Self::GetOutput) drain, the T9 navigation
/// [`StartBattle`](Self::StartBattle) (defined now so T9 does not churn the envelope),
/// and the CONTENT EDITOR's per-family query pair
/// ([`GetEditorQueryOptions`](Self::GetEditorQueryOptions) /
/// [`QueryEditor`](Self::QueryEditor), ADR 0007).
/// An independent serde enum.
///
/// One enum serves BOTH hosts — the game's `net_qa` server and the editor's — so a QA
/// client speaks one vocabulary to either. Each host services the requests it has a model
/// for and answers the rest [`BadRequest`](crate::envelope::QaError::BadRequest).
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
    /// Activate one enumerated menu item by the [`FocusTargetNet`] token a
    /// [`MenuView`](crate::view::MenuView) handed out — the generic "click a menu button
    /// by reference" act (GTW-787).
    ///
    /// It activates the item through the game's real focus-activation path (the same
    /// message an `Enter` keypress raises while the item holds focus), never a bespoke
    /// state write. A token that does not resolve to a live, listed menu item is answered
    /// [`Rejected`](crate::envelope::MenuActivationReceipt::Rejected)`(`[`StaleToken`](crate::envelope::RejectReason::StaleToken)`)`
    /// — never a panic, never a silent no-op.
    ActivateMenuItem(FocusTargetNet),
    /// Drive the focus of ANY focus-navigable screen — move focus a step, point it at one
    /// enumerated control, or activate the focused / named control (GTW-802).
    ///
    /// The generic counterpart to [`ActivateMenuItem`](Self::ActivateMenuItem): where that
    /// one needs a screen to carry the menu markers, this one drives whatever the game's own
    /// focus graph lists, so the Options screen (and any future `bevy_ui` screen) is
    /// drivable with no per-screen wiring. Unlike [`Inject`](Self::Inject) it is NOT
    /// battle-gated — a focus-navigable screen is usually off-battle.
    ///
    /// Every command is realised through the real path: a
    /// [`Step`](crate::envelope::FocusStepNet) writes the SAME navigate message an arrow key
    /// writes, and an activation emits a real `Enter` keypress at the focused control, so the
    /// game's own focus bridges and widget observers react exactly as they do to a player.
    /// A token that names no currently listed focusable is answered
    /// [`Rejected`](crate::envelope::FocusControlReceipt::Rejected)`(`[`StaleToken`](crate::envelope::RejectReason::StaleToken)`)`
    /// — never a panic, never a silent no-op.
    FocusControl(FocusCommandNet),
    /// Ask the CONTENT EDITOR which query topics it will service right now (GTW-805).
    ///
    /// The discovery half of ADR 0007's per-family query pair: the reply
    /// ([`EditorQueryOptions`](crate::envelope::QaResponse::EditorQueryOptions)) lists the
    /// live [`EditorQueryKind`]s with a one-line description each, filtered by the editor's
    /// own `EditorState` — the same filter pattern
    /// [`AppFlowView::available`](crate::view::AppFlowView::available) runs over
    /// [`RequestKindNet`]. It also carries the editor's `Load` / `Editing` readiness, so a
    /// client can poll it during the editor's asset pass and learn when the editor is ready
    /// instead of racing it.
    ///
    /// Serviced by the EDITOR host only; the game host answers it
    /// [`BadRequest`](crate::envelope::QaError::BadRequest) — it runs no editor.
    GetEditorQueryOptions,
    /// Ask the CONTENT EDITOR about ONE topic (GTW-805).
    ///
    /// The read half of the pair: the reply
    /// ([`EditorQuery`](crate::envelope::QaResponse::EditorQuery)) carries that topic's
    /// small, purpose-scoped view plus the editor's readiness. A topic the editor is not
    /// servicing right now — the model-backed topics during its `Load` asset pass — is
    /// answered [`BadRequest`](crate::envelope::QaError::BadRequest), never a fabricated
    /// empty view; poll [`GetEditorQueryOptions`](Self::GetEditorQueryOptions) first.
    ///
    /// Serviced by the EDITOR host only; the game host answers it
    /// [`BadRequest`](crate::envelope::QaError::BadRequest).
    QueryEditor(EditorQueryKind),
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
            Self::ActivateMenuItem(_) => RequestKindNet::ActivateMenuItem,
            Self::FocusControl(_) => RequestKindNet::FocusControl,
            Self::GetEditorQueryOptions => RequestKindNet::GetEditorQueryOptions,
            Self::QueryEditor(_) => RequestKindNet::QueryEditor,
        }
    }
}
