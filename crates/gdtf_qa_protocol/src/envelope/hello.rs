//! The handshake types — [`ProtocolVersion`], [`ServerNameNet`], [`HelloFacts`]
//! (GTW-734).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// The wire **protocol version** — bumped on any breaking envelope change.
///
/// A [`Hello`](crate::envelope::QaRequest::Hello) carries the client's version; the
/// server replies [`HelloOk`](crate::envelope::QaResponse::HelloOk) on a match or
/// [`VersionMismatch`](crate::envelope::QaError::VersionMismatch) otherwise. A private-
/// inner newtype (no-bare-types), serde-transparent over `u32`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolVersion(u32);

impl ProtocolVersion {
    /// The wire protocol version this build of the contract speaks.
    ///
    /// Bumped to `2` when [`AppFlowView`](crate::view::AppFlowView) gained its `available`
    /// affordance list (GTW-746) — an existing wire shape changed, so a client negotiating
    /// the old version `1` now gets a
    /// [`VersionMismatch`](crate::envelope::QaError::VersionMismatch) rather than a snapshot
    /// missing the field. Bumped to `3` when
    /// [`QaRequest::ScreenshotAfter`](crate::envelope::QaRequest::ScreenshotAfter) /
    /// [`QaResponse::ScreenshotAfter`](crate::envelope::QaResponse::ScreenshotAfter) /
    /// [`RequestKindNet::ScreenshotAfter`](crate::view::RequestKindNet::ScreenshotAfter)
    /// were added (GTW-749) — a new closed-enum variant everywhere the envelope matches
    /// exhaustively, so an old client negotiating version `2` gets a `VersionMismatch`
    /// rather than a wire shape it cannot decode. Bumped to `4` (GTW-727) for TWO breaking
    /// changes at once: [`AppFlowView`](crate::view::AppFlowView) gained a `caught_up` fact
    /// (the same class of field addition as the 1 → 2 bump) and
    /// [`QaError`](crate::envelope::QaError) gained a `NotCaughtUp` variant (the same class
    /// of closed-enum addition as the 2 → 3 bump). Negotiation is exact equality with no
    /// capability handshake, so leaving this at `3` would let a version-3 client negotiate
    /// SUCCESSFULLY and then fail to decode the very reply it asked for. Bumped to `5`
    /// (GTW-763) when [`FogView`](crate::view::FogView) changed from full `visible` /
    /// `explored` cell LISTS to `visible_count` / `explored_count` COUNTS — an existing wire
    /// shape changed (the same class as the 1 → 2 field change), so a client negotiating
    /// version `4` now gets a `VersionMismatch` rather than a `BattleView` reply whose fog it
    /// decodes against the old list shape and fails on ("frame payload was not valid compact
    /// RON"). Bumped to `6` (GTW-766) when
    /// [`QaRequest::StepperControl`](crate::envelope::QaRequest::StepperControl) /
    /// [`QaResponse::StepperControlled`](crate::envelope::QaResponse::StepperControlled) /
    /// [`RequestKindNet::StepperControl`](crate::view::RequestKindNet::StepperControl) were
    /// added — together with the
    /// [`StepperCommandNet`](crate::envelope::StepperCommandNet) command enum, the
    /// [`StepperReceipt`](crate::envelope::StepperReceipt) reply, and the
    /// [`QaError::StepperInactive`](crate::envelope::QaError::StepperInactive) error — a new
    /// closed-enum variant everywhere the envelope matches exhaustively, so an old client
    /// negotiating version `5` gets a `VersionMismatch` rather than a wire shape it cannot
    /// decode. Bumped to `7` (GTW-789) when [`BattleView`](crate::view::BattleView) gained a
    /// `buttons` field — the focus-navigable HUD button token handout
    /// ([`PanelButtonView`](crate::view::PanelButtonView)) — an existing response shape a
    /// client decodes changed (the same class of field addition as the 1 → 2 `available`
    /// and 3 → 4 `caught_up` bumps), so a client negotiating version `6` now gets a
    /// `VersionMismatch` rather than a `BattleView` reply it decodes against the old
    /// button-less shape and fails on. Bumped to `8` (GTW-787) for TWO breaking changes at
    /// once: [`AppFlowView`](crate::view::AppFlowView) gained a `menu` field — the generic
    /// menu enumeration handout ([`MenuView`](crate::view::MenuView)), the same class of
    /// field addition as the `available` / `caught_up` / `buttons` bumps — and
    /// [`QaRequest::ActivateMenuItem`](crate::envelope::QaRequest::ActivateMenuItem) /
    /// [`QaResponse::MenuItemActivated`](crate::envelope::QaResponse::MenuItemActivated) /
    /// [`RequestKindNet::ActivateMenuItem`](crate::view::RequestKindNet::ActivateMenuItem)
    /// (with the [`MenuActivationReceipt`](crate::envelope::MenuActivationReceipt) reply)
    /// were added — a new closed-enum variant everywhere the envelope matches exhaustively,
    /// so a client negotiating version `7` now gets a `VersionMismatch` rather than a wire
    /// shape it cannot decode. Bumped to `9` (GTW-816) for two more breaking changes at
    /// once: [`BattleView`](crate::view::BattleView) gained a `ui_stack` field — the DEV
    /// UI-stack swap harness's state, the same class of field addition as the `buttons` /
    /// `menu` bumps — and `NetIntent::SwapUiStack` was added alongside its `UiStackNet`
    /// payload, the
    /// [`RejectReason::Unavailable`](crate::envelope::RejectReason::Unavailable) receipt
    /// and the harness's `F9` shortcut key (all of them removed again at version `10`),
    /// so a client negotiating version `8` now gets a `VersionMismatch` rather than a
    /// `BattleView` reply it decodes against the old stack-less shape and fails on.
    /// Bumped to `10` (GTW-864) when that entire version-9 addition was REMOVED again: the
    /// DEV UI-stack swap harness it served was comparison scaffolding for a programme the
    /// 2026-07-25 two-stacks-with-a-hard-boundary ruling stood down, so `BattleView` lost
    /// its `ui_stack` field, `NetIntent` lost its swap variant, and the wire key vocabulary
    /// lost the harness's shortcut key — a removal is as breaking as an addition, so a
    /// client negotiating version `9` now gets a `VersionMismatch` rather than a reply it
    /// decodes against the stack-bearing shape and fails on. Bumped to `11` (GTW-802) for
    /// two breaking changes at once: [`AppFlowView`](crate::view::AppFlowView) gained a
    /// `focus` field — the focus-navigable control enumeration handout
    /// ([`FocusView`](crate::view::FocusView)), the same class of field addition as the
    /// `available` / `caught_up` / `menu` bumps — and
    /// [`QaRequest::FocusControl`](crate::envelope::QaRequest::FocusControl) /
    /// [`QaResponse::FocusControlled`](crate::envelope::QaResponse::FocusControlled) /
    /// [`RequestKindNet::FocusControl`](crate::view::RequestKindNet::FocusControl) were
    /// added (with the [`FocusCommandNet`](crate::envelope::FocusCommandNet) command enum
    /// and the [`FocusControlReceipt`](crate::envelope::FocusControlReceipt) reply) — a new
    /// closed-enum variant everywhere the envelope matches exhaustively, so a client
    /// negotiating version `10` now gets a `VersionMismatch` rather than a wire shape it
    /// cannot decode. Bumped to `12` (GTW-805) for the content editor's ADR 0007 query
    /// pair: [`QaRequest::GetEditorQueryOptions`](crate::envelope::QaRequest::GetEditorQueryOptions)
    /// / [`QaRequest::QueryEditor`](crate::envelope::QaRequest::QueryEditor),
    /// [`QaResponse::EditorQueryOptions`](crate::envelope::QaResponse::EditorQueryOptions) /
    /// [`QaResponse::EditorQuery`](crate::envelope::QaResponse::EditorQuery), and the two
    /// matching [`RequestKindNet`](crate::view::RequestKindNet) kinds — new closed-enum
    /// variants everywhere the envelope matches exhaustively, so a client negotiating
    /// version `11` now gets a `VersionMismatch` rather than a wire shape it cannot decode.
    /// BOTH servers — the game's and the editor's — negotiate a `Hello` against this value.
    pub const CURRENT: Self = Self::new(12);

    /// Build a protocol version from its number.
    #[must_use]
    pub const fn new(version: u32) -> Self {
        Self(version)
    }
}

/// The server's self-identifying **name** returned in the handshake (e.g. the game
/// build id) — so a client can log what it connected to.
///
/// A name newtype over `String` (no-bare-types), serde-transparent. `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ServerNameNet(String);

impl ServerNameNet {
    /// Build a server name from its identifying string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// The facts the server hands back on a successful handshake — the negotiated
/// [`protocol`](Self::protocol) version and the [`server`](Self::server) identity.
///
/// The [`HelloOk`](crate::envelope::QaResponse::HelloOk) payload. A struct (not a bare
/// version) so the handshake can grow more negotiated facts without a wire break. Serde
/// default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HelloFacts {
    /// The protocol version the server speaks.
    pub protocol: ProtocolVersion,
    /// The server's self-identifying name.
    pub server:   ServerNameNet,
}

impl HelloFacts {
    /// Build the handshake facts from the protocol version and server name.
    #[must_use]
    pub const fn new(protocol: ProtocolVersion, server: ServerNameNet) -> Self {
        Self { protocol, server }
    }
}
