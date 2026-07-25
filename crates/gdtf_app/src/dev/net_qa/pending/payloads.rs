//! The typed pending-request payloads (GTW-736; split out of the queue machinery per
//! module-layout).
//!
//! One payload type per request kind the router cannot answer synchronously, each carrying
//! the request's arguments for its LATER consumer (T4-T7 / T9 / T15 / GTW-766 / GTW-787) to
//! read. The manual `Debug` impls are deliberate (see the note above them). The
//! [`PendingQueues`](super::bundle::PendingQueues) bundle over these lives in the sibling
//! [`bundle`](super::bundle) module; the queue that holds them and its frame-deadline sweep
//! are the host-agnostic transport's ([`gdtf_net_qa_transport`], GTW-803).

use gdtf_qa_protocol::{
    envelope::{FocusCommandNet, StepperCommandNet},
    ids::{EventCap, FocusTargetNet, FrameDelay, SeedNet, ShotName, SituationRef},
    intent::NetIntent,
};

/// The pending payload for a [`GetBattleState`](gdtf_qa_protocol::envelope::QaRequest::GetBattleState)
/// snapshot — consumed by the T5 snapshot child. Carries no data (the request has no
/// arguments); it exists so the snapshot queue is a distinct type.
#[derive(Debug)]
pub(in crate::dev::net_qa) struct SnapshotPayload;

/// The pending payload for an [`Inject`](gdtf_qa_protocol::envelope::QaRequest::Inject) —
/// consumed by the T4 intent-injection child.
pub(in crate::dev::net_qa) struct InjectPayload(NetIntent);

/// The pending payload for a [`GetOutput`](gdtf_qa_protocol::envelope::QaRequest::GetOutput)
/// event drain — consumed by the T6 outbox child. The optional cap the client asked for.
pub(in crate::dev::net_qa) struct OutputPayload(Option<EventCap>);

/// The pending payload for a [`TakeScreenshot`](gdtf_qa_protocol::envelope::QaRequest::TakeScreenshot)
/// — consumed by the T7 screenshot child. The optional file stem the client asked for.
pub(in crate::dev::net_qa) struct ScreenshotPayload(Option<ShotName>);

/// The pending payload for a
/// [`ScreenshotAfter`](gdtf_qa_protocol::envelope::QaRequest::ScreenshotAfter) —
/// consumed by the T15 screenshot-after child: the embedded intent to inject, how many
/// frames to wait after it queues before capturing, and the optional file stem.
pub(in crate::dev::net_qa) struct ScreenshotAfterPayload {
    /// The intent to inject before counting down to the capture.
    intent:      NetIntent,
    /// How many frames to wait, after the intent queues, before capturing.
    frame_delay: FrameDelay,
    /// The screenshot's file stem, or `None` for a server-chosen name.
    name:        Option<ShotName>,
}

/// The pending payload for a [`StartBattle`](gdtf_qa_protocol::envelope::QaRequest::StartBattle)
/// — consumed by the T9 navigation child.
pub(in crate::dev::net_qa) struct StartBattlePayload {
    /// The situation to start.
    situation: SituationRef,
    /// The seed to pin, or `None` for a server-chosen seed.
    seed:      Option<SeedNet>,
}

/// The pending payload for a
/// [`StepperControl`](gdtf_qa_protocol::envelope::QaRequest::StepperControl) — consumed by
/// the [`drive_stepper_control`](super::super::stepper::drive_stepper_control) dispatch
/// (GTW-766). The DEV stepper-drive command to write into the panel's own latch.
pub(in crate::dev::net_qa) struct StepperControlPayload(StepperCommandNet);

/// The pending payload for an
/// [`ActivateMenuItem`](gdtf_qa_protocol::envelope::QaRequest::ActivateMenuItem) — consumed
/// by the [`drive_activate_menu_item`](super::super::activate_menu::drive_activate_menu_item)
/// consumer (GTW-787). The focus-target token naming the menu item to activate.
pub(in crate::dev::net_qa) struct ActivateMenuPayload(FocusTargetNet);

/// The pending payload for a
/// [`FocusControl`](gdtf_qa_protocol::envelope::QaRequest::FocusControl) — consumed by the
/// [`drive_focus_control`](super::super::focus_control::drive_focus_control) consumer
/// (GTW-802). The focus-drive command to realise through the game's real focus / input
/// path.
pub(in crate::dev::net_qa) struct FocusControlPayload(FocusCommandNet);

impl InjectPayload {
    /// Wrap the injected intent.
    pub(in crate::dev::net_qa) const fn new(intent: NetIntent) -> Self {
        Self(intent)
    }

    /// The wrapped intent — the T4 [`apply_injects`](super::super::inject::apply_injects)
    /// consumer's read ([`NetIntent`] is `Copy`, so this borrows without consuming).
    pub(in crate::dev::net_qa) const fn intent(&self) -> NetIntent {
        self.0
    }
}

impl OutputPayload {
    /// Wrap the optional drain cap.
    pub(in crate::dev::net_qa) const fn new(cap: Option<EventCap>) -> Self {
        Self(cap)
    }

    /// The optional per-drain event cap the client asked for — the T6
    /// [`drive_output`](super::super::events::drive_output) consumer's read
    /// ([`EventCap`] is `Copy`, so this borrows without consuming).
    pub(in crate::dev::net_qa) const fn max(&self) -> Option<EventCap> {
        self.0
    }
}

impl ScreenshotPayload {
    /// Wrap the optional screenshot stem.
    pub(in crate::dev::net_qa) const fn new(name: Option<ShotName>) -> Self {
        Self(name)
    }

    /// The wrapped stem — the T7
    /// [`drive_screenshots`](super::super::screenshot::drive_screenshots) consumer's read
    /// (borrows without consuming; the pump confines it into a path).
    pub(in crate::dev::net_qa) const fn name(&self) -> Option<&ShotName> {
        self.0.as_ref()
    }
}

impl ScreenshotAfterPayload {
    /// Pair the embedded intent with its frame delay and optional stem.
    pub(in crate::dev::net_qa) const fn new(
        intent: NetIntent,
        frame_delay: FrameDelay,
        name: Option<ShotName>,
    ) -> Self {
        Self {
            intent,
            frame_delay,
            name,
        }
    }

    /// The embedded intent — the T15 consumer's read ([`NetIntent`] is `Copy`, so this
    /// borrows without consuming).
    pub(in crate::dev::net_qa) const fn intent(&self) -> NetIntent {
        self.intent
    }

    /// How many frames to wait, after the intent queues, before capturing.
    pub(in crate::dev::net_qa) const fn frame_delay(&self) -> FrameDelay {
        self.frame_delay
    }

    /// The wrapped stem — borrows without consuming.
    pub(in crate::dev::net_qa) const fn name(&self) -> Option<&ShotName> {
        self.name.as_ref()
    }
}

impl StartBattlePayload {
    /// Pair the situation with its optional seed.
    pub(in crate::dev::net_qa) const fn new(
        situation: SituationRef,
        seed: Option<SeedNet>,
    ) -> Self {
        Self { situation, seed }
    }

    /// The situation the client asked to start — the T9
    /// [`drive_start_battle`](super::super::start_battle::drive_start_battle) consumer resolves
    /// it against the shipped-situation catalog (borrows without consuming).
    pub(in crate::dev::net_qa) const fn situation(&self) -> &SituationRef {
        &self.situation
    }

    /// The seed the client asked to pin, or `None` for a server-chosen seed — the T9
    /// consumer maps it onto a [`BattleSeed`](gdtf_battle_sim::rng::BattleSeed) override
    /// ([`SeedNet`] is `Copy`, so this reads without consuming).
    pub(in crate::dev::net_qa) const fn seed(&self) -> Option<SeedNet> {
        self.seed
    }
}

impl ActivateMenuPayload {
    /// Wrap the menu-item activation token.
    pub(in crate::dev::net_qa) const fn new(token: FocusTargetNet) -> Self {
        Self(token)
    }

    /// The wrapped token — the
    /// [`drive_activate_menu_item`](super::super::activate_menu::drive_activate_menu_item)
    /// consumer's read ([`FocusTargetNet`] is `Copy`, so this borrows without consuming).
    pub(in crate::dev::net_qa) const fn token(&self) -> FocusTargetNet {
        self.0
    }
}

impl FocusControlPayload {
    /// Wrap the focus-drive command.
    pub(in crate::dev::net_qa) const fn new(command: FocusCommandNet) -> Self {
        Self(command)
    }

    /// The wrapped focus-drive command — the
    /// [`drive_focus_control`](super::super::focus_control::drive_focus_control) consumer's
    /// read ([`FocusCommandNet`] is `Copy`, so this borrows without consuming).
    pub(in crate::dev::net_qa) const fn command(&self) -> FocusCommandNet {
        self.0
    }
}

impl StepperControlPayload {
    /// Wrap the stepper-drive command.
    pub(in crate::dev::net_qa) const fn new(command: StepperCommandNet) -> Self {
        Self(command)
    }

    /// The wrapped stepper-drive command — the dispatch's read
    /// ([`StepperCommandNet`] is `Copy`). Only the `dev_tools` dispatch body reads it (it
    /// maps the command onto the stepper's latch); the `not(dev_tools)` fallback drains and
    /// answers `Inactive` without inspecting it, so this accessor is gated to that feature.
    #[cfg(feature = "dev_tools")]
    pub(in crate::dev::net_qa) const fn command(&self) -> StepperCommandNet {
        self.0
    }
}

// Manual `Debug` impls (NOT derived) so the sweep's timeout diagnostic — the sole T3
// reader of these forward-declared payloads — genuinely reads each field: a derived
// `Debug` is ignored by dead-code analysis, an explicit `self.field` read is not.
impl core::fmt::Debug for InjectPayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("InjectPayload").field(&self.0).finish()
    }
}

impl core::fmt::Debug for OutputPayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("OutputPayload").field(&self.0).finish()
    }
}

impl core::fmt::Debug for ScreenshotPayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ScreenshotPayload").field(&self.0).finish()
    }
}

impl core::fmt::Debug for ScreenshotAfterPayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ScreenshotAfterPayload")
            .field("intent", &self.intent)
            .field("frame_delay", &self.frame_delay)
            .field("name", &self.name)
            .finish()
    }
}

impl core::fmt::Debug for StartBattlePayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StartBattlePayload")
            .field("situation", &self.situation)
            .field("seed", &self.seed)
            .finish()
    }
}

impl core::fmt::Debug for StepperControlPayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("StepperControlPayload")
            .field(&self.0)
            .finish()
    }
}

impl core::fmt::Debug for ActivateMenuPayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ActivateMenuPayload").field(&self.0).finish()
    }
}

impl core::fmt::Debug for FocusControlPayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("FocusControlPayload").field(&self.0).finish()
    }
}
